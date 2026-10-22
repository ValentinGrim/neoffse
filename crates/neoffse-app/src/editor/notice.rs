//! The notice an edit error becomes.

use neoffse::core::EditError;

use crate::{EditNotice, EditProblem};

/// The notice for an edit error.
///
/// The error's text never leaves this function (`docs/architecture.md#errors`):
/// the notice holds a problem kind and decimal numbers, and the screen writes
/// the sentence. `OutOfRange` carries the error's own `min` and `max`, which
/// can be a game floor and not the storage bound. `label` names the field or
/// the item the edit targeted.
#[must_use]
pub fn notice_of(error: &EditError, label: &str) -> EditNotice {
    let mut notice = problem_notice(problem_of(error), label);
    match error {
        EditError::OutOfRange { value, min, max, .. } => {
            notice.value = value.to_string().into();
            notice.min = min.to_string().into();
            notice.max = max.to_string().into();
        }
        EditError::AboveCap { value, cap, .. } => {
            notice.value = value.to_string().into();
            notice.cap = cap.to_string().into();
        }
        EditError::UnknownId { id, .. } => notice.value = id.to_string().into(),
        _ => {}
    }
    notice
}

/// A notice with no numbers.
#[must_use]
pub fn problem_notice(problem: EditProblem, label: &str) -> EditNotice {
    EditNotice {
        problem,
        label: label.into(),
        ..EditNotice::default()
    }
}

/// The info notice for `levels` grown levels: `EditProblem::StatsRaised`,
/// `value` the decimal count, no other number.
#[must_use]
pub fn growth_notice(levels: u32) -> EditNotice {
    EditNotice {
        value: levels.to_string().into(),
        ..problem_notice(EditProblem::StatsRaised, "")
    }
}

/// The info notice after a save: `EditProblem::Saved`. `value` is the backup
/// time `stamp_of` gives, or empty when the save made no backup.
#[must_use]
pub fn saved_notice(stamp: Option<&str>) -> EditNotice {
    EditNotice {
        value: stamp.unwrap_or_default().into(),
        ..problem_notice(EditProblem::Saved, "")
    }
}

/// The notice for a save that failed: `EditProblem::SaveFailed`, no number.
#[must_use]
pub fn save_failed_notice() -> EditNotice {
    problem_notice(EditProblem::SaveFailed, "")
}

/// The notice for a reload that failed: `EditProblem::ReloadFailed`, no number.
#[must_use]
pub fn reload_failed_notice() -> EditNotice {
    problem_notice(EditProblem::ReloadFailed, "")
}

fn problem_of(error: &EditError) -> EditProblem {
    match error {
        EditError::ReadOnly { .. } => EditProblem::ReadOnly,
        EditError::WrongType { .. } | EditError::UnknownField { .. } | EditError::NotACollection { .. } | EditError::PositionOutOfRange { .. } => {
            EditProblem::NotEditable
        }
        EditError::OutOfRange { .. } => EditProblem::OutOfRange,
        EditError::AboveCap { .. } => EditProblem::AboveCap,
        EditError::UnknownId { .. } => EditProblem::UnknownId,
        EditError::DuplicateRow { .. } => EditProblem::DuplicateRow,
        EditError::MissingRow { .. } => EditProblem::MissingRow,
        EditError::RollbackFailed { .. } => EditProblem::Reloaded,
        // `Format` and any variant a later version adds.
        _ => EditProblem::Internal,
    }
}
