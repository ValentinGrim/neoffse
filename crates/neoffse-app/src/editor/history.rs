//! The edit history of the open session: the counts the pending changes bar
//! shows, undo and redo, and the net change list of the review.

use super::layout::without_expert_only;
use super::model::{EditorViewModel, SessionLocks, lock_session, not_editable};
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
        self.shared.raw.settle(session.pending_count(), session.can_redo());
        History {
            pending: session.pending_count(),
            can_undo: session.can_undo(),
            can_redo: session.can_redo(),
        }
    }

    /// Undoes the last edit. `None`: it went through, or there was nothing to
    /// undo. `Some`: the notice to show. An undo that starts at the clean
    /// point makes the review show every row, since the undone edit may have
    /// sat on an expert-only page. It works in both modes: the replay runs
    /// under `StorageOnly`. The caller refreshes with `data`.
    ///
    /// A call with no save open, or while a save runs, gives a
    /// `not-editable` notice. A session lost to a failed rollback and a
    /// failed reload closes and answers `internal`.
    #[must_use]
    pub fn undo(&self) -> Option<EditNotice> {
        self.on_session("", |session| {
            let was_clean = !session.is_dirty();
            let moved = session.undo()?;
            if was_clean && moved {
                self.shared.raw.undone_past_clean();
            }
            Ok(moved)
        })
    }

    /// Redoes the last undone edit. Same answers as [`undo`](Self::undo).
    #[must_use]
    pub fn redo(&self) -> Option<EditNotice> {
        self.on_session("", |session| {
            let was_clean = !session.is_dirty();
            let moved = session.redo()?;
            if was_clean && session.is_dirty() {
                self.shared.raw.restore();
            }
            Ok(moved)
        })
    }

    /// The net difference between the clean state and the current state as
    /// review rows, in the registry's language. Empty when the edits cancel
    /// each other out. UI thread only: it decodes the clean bytes once per
    /// call and the result holds Slint strings.
    ///
    /// In the default mode the rows of the pages only expert mode edits stay
    /// out: a Level edit lists one row, not the stored places it writes (up
    /// to eight). Expert mode lists every row, and so does the default mode
    /// while an edit committed on such a page is pending, so no write goes
    /// unseen.
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
        let mut session = lock_session(&session);
        let internal = || problem_notice(EditProblem::Internal, "");
        let changes = session.changes(self.shared.language).map_err(|_| internal())?;
        self.shared.raw.settle(session.pending_count(), session.can_redo());
        if self.expert() || self.shared.raw.shown() {
            return Ok(review_rows(&changes));
        }
        let view = session.view().map_err(|_| internal())?;
        Ok(review_rows(&without_expert_only(changes, &view, &mut SessionLocks(&mut session))))
    }
}
