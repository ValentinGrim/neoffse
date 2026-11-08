//! The game interface: an edit applied the way the game would apply it.
//!
//! Caps and ID tables are always enforced, and the rules of
//! `docs/formats/pixel-remaster.md#inventory`,
//! `docs/formats/pixel-remaster.md#equipment-slots` and
//! `docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii` hold: an equipped
//! id sits in the inventory with a count of at least its wearers, a slot
//! `count` copies the inventory count, the placeholder rows stay alone, a key
//! item keeps a `count` of 1 and no hand takes the shield placeholder. A gain
//! stops where its base plus the gain reaches the cap
//! (`docs/formats/pixel-remaster.md#character-growth-in-ff-ii`, "Reading a
//! stored gain"). A `Set` of an equipment slot expands into plain edits; every
//! other edit passes as one.

use std::ops::RangeInclusive;

use neoffse_core::{CapPolicy, Edit, EditError, FieldId, GameChange, RowId, TableKind, Value};
use neoffse_data::BaseStats;

use super::binding::{Binding, Part};
use super::{Ff2Model, keywords, meta, names};
use crate::shared::derived::status_id;
use crate::shared::paths::List;
use crate::shared::plan::{Plan, run_plan};
use crate::shared::read::character_part;
use crate::shared::rules::{check_floor, check_insert_rules, check_remove, equip_plan, is_placeholder, row_locked, storage_max, wearers_of};
use crate::shared::stat::Stat;

/// The equipment slots that hold a hand: each takes a weapon or a shield
/// (`docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`).
const HANDS: RangeInclusive<u32> = 1..=2;

impl Ff2Model {
    /// Applies `edit` the way the game would and returns the plain edits it
    /// applied with their inverses.
    ///
    /// Caps and tables are always enforced (`CapPolicy::Enforce`). The game
    /// rules refuse, before any write: a `Set` of a computed or read-only
    /// field, of an equipment slot `count`, of a key-item `count` and any
    /// edit of a placeholder row of `inventory` (`ReadOnly`), a `Remove` of a
    /// row whose id a character wears (`ReadOnly`), an `Insert` into
    /// `key_items` with a `count` other than 1 (`OutOfRange` with `min` and
    /// `max` at 1), a `Set` or an `Insert` of an inventory `count` below the
    /// number of wearers (`OutOfRange` with `min` set to that number), a hand
    /// set to the shield placeholder, 208 (`UnknownId`), and a gain of
    /// maximum HP, maximum MP or a stat above `cap - base` (`AboveCap` with
    /// that `cap`; `UnknownId` for a status without a base row). A `Set` of
    /// `character.<id>.equipment.<S>` expands into the equip plan: the slot
    /// `contentId`, then an inventory `Insert` with the wearers as `count` or
    /// a raise of the `count` to the wearers (nothing when the count already
    /// meets them), then the slot `count` set to the inventory count.
    /// Equipping the placeholder of the slot is the unequip: the first and
    /// third steps only, the slot `count` copies the count of the placeholder
    /// row (omitted when the row is absent), and the item that left stays in
    /// the inventory. A `Set` of `character.<id>.spell.<chain>` is the Level
    /// edit: it expands into the plain `Set`s of the stored places of the
    /// spell (the `abilityList` entry, the key-3 dictionary copy and the old
    /// content id in each order list,
    /// `docs/formats/pixel-remaster.md#row-and-value-in-ff-ii`), and the
    /// inverse restores each place to its exact old value, a mismatched pair
    /// included. A character with no copy of the spell in the key-3
    /// dictionary gets `UnknownField` and no write. An `Insert` into `keywords`
    /// is the raw `Insert` under `Enforce` and appends, as `AddKeyWord` does
    /// (`docs/formats/pixel-remaster.md#keywords-in-ff-ii`): a position other
    /// than the end is `PositionOutOfRange`. A `Remove` of a keyword is
    /// `ReadOnly`, because the game has no method that removes one. Every
    /// other edit is one plain edit.
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
        // A step that rolls back can leave a rewritten string document
        // compact; the snapshot gives a failed plan back byte for byte.
        let snapshot = (plan.edits.len() > 1).then(|| self.document.clone());
        let result = run_plan(plan, |step, policy| self.apply(step, policy));
        if result.is_err()
            && let Some(document) = snapshot
        {
            self.document = document;
        }
        result
    }

    /// True when `apply_game` refuses every `Set` of `field`, whatever the
    /// value: a computed or read-only field (`max_hp`, `max_mp`, `job`), the
    /// `count` of an equipment slot, the `count` of a placeholder row of
    /// `inventory`, present in the list or not, the `count` of any key
    /// item (the game holds one of each,
    /// `docs/formats/pixel-remaster.md#inventory`) and every stored place of
    /// a spell: the game interface changes a spell through its level only,
    /// and the raw interface edits a place in expert mode. A count that only
    /// refuses values below the wearers is unlocked.
    ///
    /// It resolves the field and reads the tables, not the document.
    ///
    /// # Errors
    ///
    /// `UnknownField` for a path that names no declared field.
    pub fn field_locked(&self, field: &FieldId) -> Result<bool, EditError> {
        Ok(match Binding::resolve(field)? {
            Binding::Character {
                part: Part::SlotCount(_) | Part::MaxHp | Part::MaxMp | Part::Job | Part::Place(_),
                ..
            } => true,
            Binding::Count { list, key } => list == List::KeyItems || is_placeholder(self.tables, list, key),
            _ => false,
        })
    }

    /// True when the game interface refuses to remove `row` whatever the
    /// values are: a `character` row (the collection is not removable), a
    /// placeholder row of `inventory`, a row whose id a character wears, a
    /// row whose stored token is not canonical, whether by its text or by its
    /// escapes (`docs/formats/pixel-remaster.md#inventory`), or a canonical
    /// row whose stored `count` leaves the storage bound of a count (0, a
    /// negative number or above `i32::MAX`). `Remove` refuses such a row on
    /// both interfaces, because its inverse `Insert` could not store the
    /// count. Every row of `keywords` is locked: the game has no method that
    /// removes a keyword, and the raw interface removes one in expert mode.
    ///
    /// It reads the document and writes nothing.
    ///
    /// # Errors
    ///
    /// `NotACollection` for a row of another path, `MissingRow` for a row the
    /// document does not hold (a character row included), `Format` when a read
    /// fails.
    pub fn row_locked(&mut self, row: &RowId) -> Result<bool, EditError> {
        if keywords::owns(&row.collection()) {
            return keywords::row_locked(&mut self.document, row);
        }
        row_locked(&mut self.lists(), row, names::EQUIPMENT_SLOTS)
    }

    /// The plain edits of `edit`, after the game rules; no write happens.
    fn plan(&mut self, edit: &Edit) -> Result<Plan, EditError> {
        match edit {
            Edit::Set { field, value } => self.plan_set(edit, field, value),
            Edit::Insert { row, position, .. } if keywords::owns(&row.collection()) => {
                keywords::check_game_insert(&mut self.document, row, *position)?;
                Ok(vec![edit.clone()].into())
            }
            Edit::Insert { row, values, .. } => {
                check_insert_rules(&mut self.document, self.tables, row, values, names::EQUIPMENT_SLOTS)?;
                Ok(vec![edit.clone()].into())
            }
            // No game path removes a keyword (`AddKeyWord` is the only write method).
            Edit::Remove { row } if keywords::owns(&row.collection()) => Err(EditError::ReadOnly { field: row.id().clone() }),
            Edit::Remove { row } => {
                check_remove(&mut self.document, self.tables, row, names::EQUIPMENT_SLOTS)?;
                Ok(vec![edit.clone()].into())
            }
            _ => Ok(vec![edit.clone()].into()),
        }
    }

    fn plan_set(&mut self, edit: &Edit, field: &FieldId, value: &Value) -> Result<Plan, EditError> {
        if self.field_locked(field)? {
            return Err(EditError::ReadOnly { field: field.clone() });
        }
        let binding = Binding::resolve(field)?;
        let field_meta = meta::field_meta(&binding, self.tables);
        field_meta.check(field, value, CapPolicy::Enforce)?;
        let Value::Int(number) = *value else {
            return Ok(vec![edit.clone()].into());
        };
        match binding {
            Binding::Count { key, .. } => {
                let worn = wearers_of(&mut self.document, key, names::EQUIPMENT_SLOTS, field)?;
                check_floor(field, number, worn, &field_meta)?;
            }
            Binding::Character { id, part: Part::Slot(slot) } => {
                // `check` above holds the number to 0..=STORAGE_MAX, which fits a `u32`.
                let content_id = u32::try_from(number).map_err(|_| EditError::OutOfRange {
                    field: field.clone(),
                    value: number,
                    min: 0,
                    max: storage_max(&field_meta),
                })?;
                self.check_hand(field, slot, content_id)?;
                let plan = equip_plan(&mut self.document, self.tables, field, (id, slot), content_id, names::EQUIPMENT_SLOTS)?;
                return Ok(plan.into());
            }
            Binding::Character { id, part: Part::Spell(chain) } => return self.spell_plan(field, id, chain, number),
            Binding::Character {
                id,
                part: part @ (Part::MaxHpGain | Part::MaxMpGain | Part::Stat(_)),
            } => self.check_gain(field, id, part, number)?,
            // Every other edit passes as one plain edit.
            _ => {}
        }
        Ok(vec![edit.clone()].into())
    }

    /// `UnknownId` for the shield placeholder in a hand: no game file places
    /// it in a slot, and an empty hand holds 47
    /// (`docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`). The raw
    /// interface still takes it, since the Shield table lists the row.
    fn check_hand(&self, field: &FieldId, slot: u32, content_id: u32) -> Result<(), EditError> {
        let shield_placeholder = self.tables.entry(TableKind::Shield, content_id).is_some_and(|entry| entry.placeholder);
        if HANDS.contains(&slot) && shield_placeholder {
            return Err(EditError::UnknownId {
                field: field.clone(),
                id: i64::from(content_id),
            });
        }
        Ok(())
    }

    /// `AboveCap { cap: cap - base }` for a gain of character `id` whose base
    /// plus gain would pass the game cap
    /// (`docs/formats/pixel-remaster.md#character-growth-in-ff-ii`, "Reading a
    /// stored gain"). The base is the character's `character_status` row.
    /// `UnknownField` when the character lacks `characterStatusId`,
    /// `UnknownId` when no row has that status.
    fn check_gain(&mut self, field: &FieldId, id: u32, part: Part, gain: i64) -> Result<(), EditError> {
        let base = character_part(&mut self.document, id, field, |_, base| Ok(base))?;
        let Some(status) = status_id(&mut self.document, &base, field)? else {
            return Err(EditError::UnknownField { field: field.clone() });
        };
        // A status below 0 or above `u32` has no row, which is the `UnknownId` below.
        let row = u32::try_from(status).ok().and_then(|status| self.tables.base_stats(status));
        let Some(base_row) = row else {
            return Err(EditError::UnknownId {
                field: field.clone(),
                id: status,
            });
        };
        // A game without the cap key has no cap for the gain, as the field meta does.
        let Some((base, cap)) = gain_base(part, &base_row).and_then(|(base, key)| Some((base, self.tables.bound(key)?))) else {
            return Ok(());
        };
        let room = cap - i64::from(base);
        if gain > room {
            return Err(EditError::AboveCap {
                field: field.clone(),
                value: gain,
                cap: room,
            });
        }
        Ok(())
    }
}

/// The base of a gain in the status row and the `bounds.toml` key of its cap.
/// `None` for a part that is no gain, and for `Luck`, which the binding never
/// builds in FF II.
fn gain_base(part: Part, stats: &BaseStats) -> Option<(u32, &'static str)> {
    Some(match part {
        Part::MaxHpGain => (stats.hp, "MAX_HP"),
        Part::MaxMpGain => (stats.mp, "MAX_MP"),
        Part::Stat(stat) => {
            let base = match stat {
                Stat::Power => stats.power,
                Stat::Vitality => stats.vitality,
                Stat::Agility => stats.agility,
                Stat::Intelligence => stats.intelligence,
                Stat::Spirit => stats.spirit,
                Stat::Magic => stats.magic,
                Stat::Luck => return None,
            };
            (base, stat.bound_key())
        }
        _ => return None,
    })
}
