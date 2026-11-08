//! The game interface: an edit applied the way the game would apply it.
//!
//! Caps and ID tables are always enforced, and the equipment and inventory
//! rules of `docs/formats/pixel-remaster.md#inventory` and
//! `docs/formats/pixel-remaster.md#equipment-slots` hold: an equipped id sits
//! in the inventory with a count of at least its wearers, a slot `count`
//! copies the inventory count, the placeholder rows stay alone, and a key item
//! keeps a `count` of 1. A `Set` of an equipment slot expands into plain
//! edits, and so does a `Set` of Level or Experience, which each move the
//! other (`docs/formats/pixel-remaster.md#level-up-in-ff-i`); every other
//! edit passes as one.

use neoffse_core::{CapPolicy, Edit, EditError, FieldId, GameChange, Value};

use super::binding::{Binding, Part};
use super::{Ff1Model, Rolls, ThreadRolls, names};
use crate::shared::access;
use crate::shared::plan::{Plan, run_plan};
use crate::shared::rules::{check_floor, check_insert_rules, check_remove, equip_plan, wearers_of};

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
    /// A `Set` of a level or an experience also sets the other: the level
    /// that experience reaches, or the first experience point of that level
    /// (`docs/formats/pixel-remaster.md#level-up-in-ff-i`). The linked step is
    /// left out when the stored value already matches, a level 0 is
    /// `OutOfRange` with `min` 1, and raw [`apply`](Self::apply) links
    /// nothing. When the level rises, the plan ends with the growth step of
    /// each level gained, as the game applies it after a battle; that step
    /// draws random numbers from [`ThreadRolls`], and
    /// [`apply_game_with`](Self::apply_game_with) takes another source. Every
    /// other edit is one plain edit.
    ///
    /// The plan applies all or none: a failure at any step gives the document
    /// back byte for byte.
    ///
    /// # Errors
    ///
    /// The errors of [`apply`](Self::apply) under `Enforce`, plus the game
    /// rules above and `UnknownId` for a job or a status without a growth row.
    pub fn apply_game(&mut self, edit: &Edit) -> Result<GameChange, EditError> {
        self.apply_game_with(edit, &mut ThreadRolls)
    }

    /// [`apply_game`](Self::apply_game) with a caller-supplied source for the
    /// random draws of a level-up. A test scripts it; `apply_game` passes
    /// [`ThreadRolls`].
    ///
    /// The requested edit and its link apply under `CapPolicy::Enforce`. The
    /// growth edits apply under `CapPolicy::StorageOnly`: the game stores the
    /// raw sums and clamps on read, so a stat can pass its cap
    /// (`docs/formats/pixel-remaster.md#level-up-in-ff-i`, "What a step leaves
    /// alone"). The storage bounds still hold. `forward` holds the concrete
    /// edits, so `Session::undo` and `redo` give the drawn values back.
    ///
    /// # Errors
    ///
    /// The errors of `apply_game`.
    pub fn apply_game_with(&mut self, edit: &Edit, rolls: &mut dyn Rolls) -> Result<GameChange, EditError> {
        let plan = self.plan(edit, rolls)?;
        // A step that rolls back leaves its levels rewritten compact; the
        // snapshot gives a failed plan back byte for byte.
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
    /// value: a computed field (`max_hp`, `next_level`), a spell slot, the
    /// `count` of an equipment slot, the `count` of a placeholder row of
    /// `inventory`, present in the list or not, and the `count` of any key item (the game holds one of each). A count that only refuses values
    /// below the wearers is unlocked.
    ///
    /// It resolves the field and reads the tables, not the document.
    ///
    /// # Errors
    ///
    /// `UnknownField` for a path that names no declared field.
    pub fn field_locked(&self, field: &FieldId) -> Result<bool, EditError> {
        access::field_locked::<Part>(self.tables, field)
    }

    /// The plain edits of `edit`, after the game rules; no write happens.
    fn plan(&mut self, edit: &Edit, rolls: &mut dyn Rolls) -> Result<Plan, EditError> {
        match edit {
            Edit::Set { field, value } => self.plan_set(edit, field, value, rolls),
            Edit::Insert { row, values, .. } => {
                check_insert_rules(&mut self.document, self.tables, row, values, names::EQUIPMENT_SLOTS)?;
                Ok(vec![edit.clone()].into())
            }
            Edit::Remove { row } => {
                check_remove(&mut self.document, self.tables, row, names::EQUIPMENT_SLOTS)?;
                Ok(vec![edit.clone()].into())
            }
            _ => Ok(vec![edit.clone()].into()),
        }
    }

    fn plan_set(&mut self, edit: &Edit, field: &FieldId, value: &Value, rolls: &mut dyn Rolls) -> Result<Plan, EditError> {
        if self.field_locked(field)? {
            return Err(EditError::ReadOnly { field: field.clone() });
        }
        let binding = Binding::resolve(field)?;
        let field_meta = binding.meta(self.tables);
        field_meta.check(field, value, CapPolicy::Enforce)?;
        let Value::Int(number) = *value else {
            return Ok(vec![edit.clone()].into());
        };
        match binding {
            Binding::Count { key, .. } => {
                let worn = wearers_of(&mut self.document, key, names::EQUIPMENT_SLOTS, field)?;
                check_floor(field, number, worn, &field_meta)?;
                Ok(vec![edit.clone()].into())
            }
            Binding::Character { id, part: Part::Level } => self.level_plan(id, number, rolls),
            Binding::Character { id, part: Part::Exp } => self.exp_plan(id, number, rolls),
            Binding::Character { id, part: Part::Slot(slot) } => match u32::try_from(number) {
                Ok(content_id) => equip_plan(&mut self.document, self.tables, field, (id, slot), content_id, names::EQUIPMENT_SLOTS).map(Plan::from),
                // check() above holds the number to 0..=STORAGE_MAX, so this arm never runs; apply checks the value again.
                Err(_) => Ok(vec![edit.clone()].into()),
            },
            _ => Ok(vec![edit.clone()].into()),
        }
    }
}
