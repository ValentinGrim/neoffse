//! The edit history of the open session: the counts the pending changes bar
//! shows, undo and redo, and the net change list of the review.

use neoffse::Session;
use neoffse::core::EditError;

use super::edit::settle;
use super::model::{EditorViewModel, lock_session, not_editable};
use super::notice::problem_notice;
use super::review::review_rows;
use crate::{EditNotice, EditProblem, ReviewRow};

/// What the history of the open session lets the user do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct History {
    /// How far the session is from its clean point: the unsaved changes the
    /// bar counts. Undoing back to the clean point gives 0.
    pub pending: usize,
    /// Whether an undo step exists.
    pub can_undo: bool,
    /// Whether a redo step exists.
    pub can_redo: bool,
}

impl EditorViewModel {
    /// The history counts of the open save. The default (nothing pending, no
    /// step) when no save is open.
    ///
    /// The call locks the session. It waits for a running save, so the UI
    /// glue skips it while [`is_saving`](Self::is_saving).
    #[must_use]
    pub fn history(&self) -> History {
        let Some(session) = self.shared.state.session() else {
            return History::default();
        };
        let session = lock_session(&session);
        History {
            pending: session.pending_count(),
            can_undo: session.can_undo(),
            can_redo: session.can_redo(),
        }
    }

    /// Undoes the last edit. `None`: it went through, or there was nothing to
    /// undo. `Some`: the notice to show. It works in both modes: the replay
    /// runs under `StorageOnly`. The caller refreshes with `data`.
    ///
    /// A call with no save open, or while a save runs, gives a
    /// `not-editable` notice.
    #[must_use]
    pub fn undo(&self) -> Option<EditNotice> {
        self.replay(Session::undo)
    }

    /// Redoes the last undone edit. Same answers as [`undo`](Self::undo).
    #[must_use]
    pub fn redo(&self) -> Option<EditNotice> {
        self.replay(Session::redo)
    }

    /// The net difference between the clean state and the current state as
    /// review rows, in the registry's language. Empty when the edits cancel
    /// each other out. UI thread only: it decodes the clean bytes once per
    /// call and the result holds Slint strings.
    ///
    /// # Errors
    ///
    /// A `not-editable` notice when no save is open or a save runs, an
    /// `internal` notice when the comparison fails.
    pub fn review(&self) -> Result<Vec<ReviewRow>, EditNotice> {
        if self.is_saving() {
            return Err(not_editable());
        }
        let session = self.shared.state.session().ok_or_else(not_editable)?;
        let changes = lock_session(&session)
            .changes(self.shared.language)
            .map_err(|_| problem_notice(EditProblem::Internal, ""))?;
        Ok(review_rows(&changes))
    }

    fn replay(&self, step: impl FnOnce(&mut Session) -> Result<bool, EditError>) -> Option<EditNotice> {
        if self.is_saving() {
            return Some(not_editable());
        }
        let Some(session) = self.shared.state.session() else {
            return Some(not_editable());
        };
        let mut session = lock_session(&session);
        let error = step(&mut session).err()?;
        Some(settle(&mut session, &error, ""))
    }
}
