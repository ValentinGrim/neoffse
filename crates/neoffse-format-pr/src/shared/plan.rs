//! The plan of a game-interface edit and the runner that applies it, plus the
//! checks every game runs before it plans: the lock, the meta, the wearer
//! floor of a count, and the insert and remove rules of the item lists.

use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FieldId, FieldMeta, GameChange, Value};
use neoffse_data::GameTables;

use super::access::field_locked;
use super::bind::{Binding, CharacterPart};
use super::model::SlotModel;
use super::rules::{check_floor, check_insert_rules, check_remove, wearers_of};
use crate::document::Document;

/// The edits of a game-interface edit, in apply order.
pub(crate) struct Plan {
    pub(crate) edits: Vec<Edit>,
    /// The number of leading edits that apply under `CapPolicy::Enforce`: the
    /// requested edit and its link. The rest are growth edits, which the game
    /// stores as raw sums (`docs/formats/pixel-remaster.md#level-up-in-ff-i`,
    /// "What a step leaves alone").
    pub(crate) enforced: usize,
}

impl From<Vec<Edit>> for Plan {
    fn from(edits: Vec<Edit>) -> Self {
        let enforced = edits.len();
        Self { edits, enforced }
    }
}

impl Plan {
    /// The plan that applies `edit` as it stands.
    pub(crate) fn single(edit: &Edit) -> Self {
        vec![edit.clone()].into()
    }
}

/// Applies `plan` through `apply` (the raw edit of the model) and returns the
/// concrete edits with their inverses. A failed step rolls the earlier ones
/// back; a model whose steps rewrite more than they undo also restores a
/// snapshot of its document, which it takes before the call.
pub(crate) fn run_plan<F>(plan: Plan, mut apply: F) -> Result<GameChange, EditError>
where
    F: FnMut(&Edit, CapPolicy) -> Result<Edit, EditError>,
{
    let Plan { edits, enforced } = plan;
    let forward: ChangeSet = edits.into_iter().collect();
    let mut applied = 0;
    // `apply_with` rolls back under `StorageOnly`; only a forward call
    // arrives under `Enforce`.
    let inverse = forward.apply_with(CapPolicy::Enforce, |step, policy| {
        let growing = policy == CapPolicy::Enforce && applied >= enforced;
        applied += 1;
        apply(step, if growing { CapPolicy::StorageOnly } else { policy })
    })?;
    Ok(GameChange::new(forward, inverse))
}

/// Applies `plan` to `model` through its raw `apply`, all or none. A plan of
/// two edits or more takes a snapshot of the model first: a step that rolls
/// back can leave a rewritten string document compact, and the snapshot gives
/// a failed plan back byte for byte.
pub(crate) fn run_game<M: SlotModel>(model: &mut M, plan: Plan) -> Result<GameChange, EditError> {
    let snapshot = (plan.edits.len() > 1).then(|| model.clone());
    let result = run_plan(plan, |step, policy| model.apply(step, policy));
    if result.is_err()
        && let Some(saved) = snapshot
    {
        *model = saved;
    }
    result
}

/// What the shared checks leave a game to plan.
pub(crate) enum Head<P> {
    /// Nothing game-specific: the edit applies as one plain edit.
    Plain,
    /// A `Set` of a character part with an integer value that passed the
    /// checks; `meta` is the meta of the field.
    Character { id: u32, part: P, number: i64, meta: FieldMeta },
}

/// The checks that open every `Set` of the game interface, in this order: the
/// lock (`ReadOnly`), the field (`UnknownField`), the meta under `Enforce`,
/// the integer test, then the wearer floor of an inventory count
/// (`docs/formats/pixel-remaster.md#inventory`). A character part with an
/// integer value goes back to the game, which owns its rules.
pub(crate) fn plan_head<P: CharacterPart>(
    document: &mut Document,
    tables: &GameTables,
    field: &FieldId,
    value: &Value,
    slots: u32,
) -> Result<Head<P>, EditError> {
    if field_locked::<P>(tables, field)? {
        return Err(EditError::ReadOnly { field: field.clone() });
    }
    let binding = Binding::<P>::resolve(field)?;
    let meta = binding.meta(tables);
    meta.check(field, value, CapPolicy::Enforce)?;
    let Value::Int(number) = *value else {
        return Ok(Head::Plain);
    };
    match binding {
        Binding::Count { key, .. } => {
            let worn = wearers_of(document, key, slots, field)?;
            check_floor(field, number, worn, &meta)?;
            Ok(Head::Plain)
        }
        Binding::Character { id, part } => Ok(Head::Character { id, part, number, meta }),
        Binding::Gil => Ok(Head::Plain),
    }
}

/// The plan of an `Insert`, a `Remove` or any other edit that is no `Set`:
/// the item-list rules run before the edit passes as one.
pub(crate) fn item_plan(document: &mut Document, tables: &GameTables, edit: &Edit, slots: u32) -> Result<Plan, EditError> {
    match edit {
        Edit::Insert { row, values, .. } => check_insert_rules(document, tables, row, values, slots)?,
        Edit::Remove { row } => check_remove(document, tables, row, slots)?,
        // `Edit` is `#[non_exhaustive]`; both callers plan a `Set` first, so no edit
        // reaches this arm today. A later variant would pass as one plain edit.
        _ => {}
    }
    Ok(Plan::single(edit))
}
