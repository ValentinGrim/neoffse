//! The link between Level and Experience under the game interface.
//!
//! An Experience edit also sets the level it reaches, and a Level edit also
//! sets the first experience point of that level
//! (`docs/formats/pixel-remaster.md#level-up-in-ff-i`, "Level and Experience in
//! NeoFFSE"). Both plans put the requested edit first and leave the linked
//! step out when the stored value already matches. Stats do not move.

use neoffse_core::{Edit, EditError, FieldId, Value};

use super::derived::level_for;
use super::{Ff1Model, names};

/// The field `name` of character `id`.
fn character_field(id: u32, name: &'static str) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id).with_name(name)
}

impl Ff1Model {
    /// The plan of `Set character.<id>.level` to `level`: the level, then the
    /// experience at which that level starts.
    ///
    /// `OutOfRange` with `min` 1 for level 0 and for a level with no row in
    /// the experience table; `max` is the cap `MAX_LV`, or the last level
    /// with a row when the cap is missing. The check runs before any write.
    pub(super) fn level_plan(&mut self, id: u32, level: i64) -> Result<Vec<Edit>, EditError> {
        let field = character_field(id, names::LEVEL);
        let first = u32::try_from(level).ok().and_then(|level| self.tables.exp_to_reach(level));
        let Some(first) = first.filter(|_| level >= 1) else {
            let last = (1..).map_while(|level| self.tables.exp_to_reach(level)).count();
            let last = i64::try_from(last).unwrap_or(i64::MAX);
            return Err(EditError::OutOfRange {
                field,
                value: level,
                min: 1,
                max: self.tables.bound("MAX_LV").unwrap_or(last),
            });
        };
        let mut plan = vec![Edit::set(field, Value::Int(level))];
        self.push_if_changed(&mut plan, character_field(id, names::EXP), i64::from(first))?;
        Ok(plan)
    }

    /// The plan of `Set character.<id>.exp` to `exp`: the experience, then the
    /// level it reaches, at most the last row of the table.
    pub(super) fn exp_plan(&mut self, id: u32, exp: i64) -> Result<Vec<Edit>, EditError> {
        let mut plan = vec![Edit::set(character_field(id, names::EXP), Value::Int(exp))];
        let reached = level_for(self.tables, u32::try_from(exp).unwrap_or(u32::MAX));
        if let Some(reached) = reached {
            self.push_if_changed(&mut plan, character_field(id, names::LEVEL), i64::from(reached))?;
        }
        Ok(plan)
    }

    /// Appends `Set linked = target` unless the stored value already equals it.
    fn push_if_changed(&mut self, plan: &mut Vec<Edit>, linked: FieldId, target: i64) -> Result<(), EditError> {
        let Value::Int(stored) = self.get(&linked)? else {
            return Err(EditError::ReadOnly { field: linked });
        };
        if stored != target {
            plan.push(Edit::set(linked, Value::Int(target)));
        }
        Ok(())
    }
}
