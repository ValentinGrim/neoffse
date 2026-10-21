//! Leaving the editor: the question a dirty session asks before it closes,
//! and the offer to switch to expert mode after an above-cap refusal.

use std::sync::PoisonError;

use super::model::EditorViewModel;
use crate::{EditNotice, EditProblem};

/// Why the user leaves the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Leave {
    /// The Back button: the save closes, the window stays.
    Back,
    /// The window closes.
    Close,
}

/// What a leave request needs next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LeaveAnswer {
    /// The save is closed (or none was open): the request goes through.
    Left,
    /// The save holds unsaved edits: show the discard dialog.
    Ask,
}

impl EditorViewModel {
    /// Leaves now when the session is clean or absent: closes it, turns
    /// expert mode off and answers `Left`. Otherwise remembers `why` and
    /// answers `Ask`. A second request while one waits replaces the first.
    #[must_use]
    pub fn leave(&self, why: Leave) -> LeaveAnswer {
        if self.is_dirty() {
            *self.pending() = Some(why);
            LeaveAnswer::Ask
        } else {
            self.back();
            *self.pending() = None;
            LeaveAnswer::Left
        }
    }

    /// The user chose Discard: closes the session, turns expert mode off and
    /// returns the remembered reason. `None` when no request was pending; the
    /// session then stays.
    #[must_use]
    pub fn confirm_discard(&self) -> Option<Leave> {
        let why = self.pending().take()?;
        self.back();
        Some(why)
    }

    /// The user chose Cancel: forgets the pending request. The session
    /// stays.
    pub fn cancel_leave(&self) {
        *self.pending() = None;
    }

    /// Whether a leave request waits for an answer.
    #[must_use]
    pub fn leave_pending(&self) -> bool {
        self.pending().is_some()
    }

    /// Whether `notice` is an above-cap refusal that expert mode would
    /// accept: the problem is `above-cap` and expert mode is off.
    #[must_use]
    pub fn offers_expert(&self, notice: &EditNotice) -> bool {
        notice.problem == EditProblem::AboveCap && !self.expert()
    }

    fn pending(&self) -> std::sync::MutexGuard<'_, Option<Leave>> {
        self.shared.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
