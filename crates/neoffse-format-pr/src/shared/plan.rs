//! The plan of a game-interface edit and the runner that applies it.

use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, GameChange};

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
