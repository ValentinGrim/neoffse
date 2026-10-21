//! The game interface: an edit applied the way the game would apply it.
//!
//! Caps and ID tables are always enforced, and the equipment and inventory
//! rules of `docs/formats/pixel-remaster.md#inventory` and
//! `docs/formats/pixel-remaster.md#equipment-slots` hold: an equipped id sits
//! in the inventory with a count of at least its wearers, a slot `count`
//! copies the inventory count, the placeholder rows stay alone, and a key item
//! keeps a `count` of 1. A `Set` of an equipment slot expands into plain
//! edits; every other edit passes as one.

use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FieldId, FieldMeta, GameChange, RowId, Shape, Value};

use super::binding::{Binding, Collection, List, Part};
use super::locate::{find_row, format_error, wearers};
use super::{Ff1Model, meta, names};

/// The upper storage bound of a count field.
fn storage_max(meta: &FieldMeta) -> i64 {
    match meta.shape {
        Shape::Int { max, .. } => max,
        _ => meta::STORAGE_MAX,
    }
}

/// `OutOfRange { min: worn }` for a count below the wearers.
fn check_floor(field: &FieldId, count: i64, worn: usize, count_meta: &FieldMeta) -> Result<(), EditError> {
    let min = i64::try_from(worn).unwrap_or(i64::MAX);
    if count < min {
        return Err(EditError::OutOfRange {
            field: field.clone(),
            value: count,
            min,
            max: storage_max(count_meta),
        });
    }
    Ok(())
}

impl Ff1Model {
    /// Applies `edit` the way the game would and returns the plain edits it
    /// applied with their inverses.
    ///
    /// Caps and tables are always enforced (`CapPolicy::Enforce`). The game
    /// rules refuse, before any write: a `Set` of an equipment slot `count`,
    /// a `Set` of a key-item `count` and any edit of a placeholder row of
    /// `inventory` (`ReadOnly`), a `Remove` of a row whose id a character
    /// wears (`ReadOnly`), an `Insert` into `key_items` with a `count` other
    /// than 1 (`OutOfRange` with `min` and `max` at 1), and a `Set` or an
    /// `Insert` of an inventory `count` below the number of wearers
    /// (`OutOfRange` with `min` set to that number). A `Set` of
    /// `character.<id>.equipment.<S>` expands into the equip plan: the slot
    /// `contentId`, then an inventory `Insert` with the wearers as `count` or
    /// a raise of the `count` to the wearers (nothing when the count already
    /// meets them), then the slot `count` set to the inventory count. Equipping
    /// the placeholder of the slot is the unequip: the first and third steps
    /// only, the slot `count` copies the count of the placeholder row (omitted
    /// when the row is absent), and the item that left stays in the inventory.
    /// Every other edit is one plain edit.
    ///
    /// The plan applies all or none: a failure at any step gives the document
    /// back byte for byte.
    ///
    /// # Errors
    ///
    /// The errors of [`apply`](Self::apply) under `Enforce`, plus the game
    /// rules above.
    pub fn apply_game(&mut self, edit: &Edit) -> Result<GameChange, EditError> {
        let plan = self.plan(edit)?;
        // A step that rolls back leaves its levels rewritten compact; the
        // snapshot gives a failed plan back byte for byte.
        let snapshot = (plan.len() > 1).then(|| self.document.clone());
        let forward: ChangeSet = plan.into_iter().collect();
        match forward.apply_with(CapPolicy::Enforce, |step, policy| self.apply(step, policy)) {
            Ok(inverse) => Ok(GameChange::new(forward, inverse)),
            Err(error) => {
                if let Some(document) = snapshot {
                    self.document = document;
                }
                Err(error)
            }
        }
    }

    /// True when `apply_game` refuses every `Set` of `field`, whatever the
    /// value: a spell slot, the `count` of an equipment slot, the `count` of a placeholder
    /// row of `inventory`, present in the list or not, and the `count` of any
    /// key item (the game holds one of each). A count that only refuses values
    /// below the wearers is unlocked.
    ///
    /// It resolves the field and reads the tables, not the document.
    ///
    /// # Errors
    ///
    /// `UnknownField` for a path that names no declared field.
    pub fn field_locked(&self, field: &FieldId) -> Result<bool, EditError> {
        Ok(match Binding::resolve(field)? {
            Binding::Character {
                part: Part::SlotCount(_) | Part::Spell { .. },
                ..
            } => true,
            Binding::Count { list, key } => list == List::KeyItems || self.is_placeholder(list, key),
            _ => false,
        })
    }

    /// A placeholder row of `inventory`: its count has no known meaning, so
    /// the game interface leaves the row alone
    /// (`docs/formats/pixel-remaster.md#inventory`). `key_items` has none.
    pub(super) fn is_placeholder(&self, list: List, key: u32) -> bool {
        list == List::Inventory && self.tables.content(key).is_some_and(|entry| entry.placeholder)
    }

    /// The number of equipment slots that hold `content_id`. A key item is
    /// never worn, so a `key_items` row that goes through this check always
    /// passes it.
    pub(super) fn wearers_of(&mut self, content_id: u32, field: &FieldId) -> Result<usize, EditError> {
        wearers(&mut self.document, content_id).map_err(|source| format_error(field, source))
    }

    /// The plain edits of `edit`, after the game rules; no write happens.
    fn plan(&mut self, edit: &Edit) -> Result<Vec<Edit>, EditError> {
        match edit {
            Edit::Set { field, value } => self.plan_set(edit, field, value),
            Edit::Insert { row, values, .. } => {
                self.check_insert_rules(row, values)?;
                Ok(vec![edit.clone()])
            }
            Edit::Remove { row } => {
                if let Some(Collection::Items(list)) = Collection::of(&row.collection()) {
                    let locked = self.is_placeholder(list, row.key()) || self.wearers_of(row.key(), row.id())? > 0;
                    if locked {
                        return Err(EditError::ReadOnly { field: row.id().clone() });
                    }
                }
                Ok(vec![edit.clone()])
            }
            _ => Ok(vec![edit.clone()]),
        }
    }

    fn plan_set(&mut self, edit: &Edit, field: &FieldId, value: &Value) -> Result<Vec<Edit>, EditError> {
        if self.field_locked(field)? {
            return Err(EditError::ReadOnly { field: field.clone() });
        }
        let binding = Binding::resolve(field)?;
        let field_meta = meta::field_meta(&binding, self.tables);
        field_meta.check(field, value, CapPolicy::Enforce)?;
        let Value::Int(number) = *value else {
            return Ok(vec![edit.clone()]);
        };
        match binding {
            Binding::Count { key, .. } => {
                let worn = self.wearers_of(key, field)?;
                check_floor(field, number, worn, &field_meta)?;
                Ok(vec![edit.clone()])
            }
            Binding::Character { part: Part::Slot(_), .. } => match u32::try_from(number) {
                Ok(content_id) => self.equip_plan(field, content_id),
                Err(_) => Ok(vec![edit.clone()]),
            },
            _ => Ok(vec![edit.clone()]),
        }
    }

    /// An `Insert` into `inventory` or `key_items`: a placeholder key is
    /// `ReadOnly`, a key-item `count` other than 1 and a `count` below the
    /// wearers are `OutOfRange`.
    fn check_insert_rules(&mut self, row: &RowId, values: &[(&'static str, Value)]) -> Result<(), EditError> {
        let collection = row.collection();
        let (Some(Collection::Items(list)), Some(meta)) = (Collection::of(&collection), self.collection_meta(&collection)) else {
            return Ok(());
        };
        if self.is_placeholder(list, row.key()) {
            return Err(EditError::ReadOnly { field: row.id().clone() });
        }
        let given = values.iter().find(|(name, _)| *name == names::COUNT).map(|(_, value)| value);
        if let (List::KeyItems, Some(Value::Int(count))) = (list, given)
            && *count != 1
        {
            return Err(EditError::OutOfRange {
                field: row.field(names::COUNT),
                value: *count,
                min: 1,
                max: 1,
            });
        }
        meta.check_insert(row, values, CapPolicy::Enforce)?;
        let Some(count_field) = meta.fields.iter().find(|field| field.name == names::COUNT) else {
            return Ok(());
        };
        if let Some(Value::Int(count)) = given.or(Some(&count_field.initial)) {
            let worn = self.wearers_of(row.key(), row.id())?;
            check_floor(&row.field(names::COUNT), *count, worn, &count_field.meta)?;
        }
        Ok(())
    }

    /// The equip plan of `Set character.<id>.equipment.<slot>` to
    /// `content_id`.
    fn equip_plan(&mut self, slot_field: &FieldId, content_id: u32) -> Result<Vec<Edit>, EditError> {
        let mut plan = vec![Edit::set(slot_field.clone(), Value::Int(i64::from(content_id)))];
        let slot_count = slot_field.clone().with_name(names::COUNT);
        let row = RowId::new(FieldId::new(names::INVENTORY), content_id);
        let count_field = row.field(names::COUNT);
        let failed = |source| format_error(&count_field, source);
        let held = find_row(&mut self.document, Collection::Items(List::Inventory), content_id).map_err(failed)?;
        let placeholder = self.is_placeholder(List::Inventory, content_id);
        if placeholder {
            if held.is_some() {
                let Value::Int(count) = self.get(&count_field)? else {
                    return Err(EditError::ReadOnly { field: count_field });
                };
                plan.push(Edit::set(slot_count, Value::Int(count)));
            }
            return Ok(plan);
        }
        let Value::Int(old) = self.get(slot_field)? else {
            return Err(EditError::ReadOnly { field: slot_field.clone() });
        };
        let others = self
            .wearers_of(content_id, slot_field)?
            .saturating_sub(usize::from(old == i64::from(content_id)));
        let needed = i64::try_from(others + 1).unwrap_or(i64::MAX);
        let copied = if held.is_none() {
            plan.push(Edit::insert(row, None, vec![(names::COUNT, Value::Int(needed))]));
            needed
        } else {
            let Value::Int(stored) = self.get(&count_field)? else {
                return Err(EditError::ReadOnly { field: count_field });
            };
            if stored < needed {
                plan.push(Edit::set(count_field, Value::Int(needed)));
                needed
            } else {
                stored
            }
        };
        plan.push(Edit::set(slot_count, Value::Int(copied)));
        Ok(plan)
    }
}
