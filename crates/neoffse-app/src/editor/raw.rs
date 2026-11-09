//! Whether the open session holds an edit committed on a page only expert
//! mode edits.
//!
//! The review leaves the rows of such pages out in the default mode, because
//! the Level edit of a spell writes up to eight stored places and the user
//! asked for one change. An edit committed on one of those pages is a change the
//! user made on purpose, so the review shows every row while one is pending.
//! The state follows the session's return to its clean point: a save, a
//! discard or a new file clears it, and an undo back to the clean point parks
//! it so the redo restores it.

use std::sync::{Mutex, MutexGuard, PoisonError};

#[derive(Debug, Default, Clone, Copy)]
struct State {
    /// An edit committed on an expert-only page sits between the clean point
    /// and the document.
    live: bool,
    /// `live` as it stood when an undo reached the clean point.
    parked: bool,
}

/// The flag of the open session. Plain data: it holds no save content.
#[derive(Debug, Default)]
pub(super) struct RawEdits {
    state: Mutex<State>,
}

impl RawEdits {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Whether the review shows every row.
    pub(super) fn shown(&self) -> bool {
        self.state().live
    }

    /// Follows the session to its clean point: `pending` is its distance to
    /// the clean point and `can_redo` tells whether a redo step waits. At the
    /// clean point with a redo step, the flag parks for the redo; without
    /// one, a save, a discard or a new file started afresh and nothing is
    /// left to restore.
    pub(super) fn settle(&self, pending: usize, can_redo: bool) {
        if pending != 0 {
            return;
        }
        let mut state = self.state();
        state.parked |= state.live;
        state.live = false;
        if !can_redo {
            state.parked = false;
        }
    }

    /// A new edit went through: the redo steps are gone. `on_expert_page`
    /// tells whether it was committed on an expert-only page.
    pub(super) fn committed(&self, on_expert_page: bool) {
        let mut state = self.state();
        state.parked = false;
        state.live |= on_expert_page;
    }

    /// An undo started at the clean point and went past it. It may reverse an
    /// edit saved on an expert-only page, and the session keeps no marker per
    /// entry, so the flag goes live: the review shows every row.
    pub(super) fn undone_past_clean(&self) {
        self.state().live = true;
    }

    /// A redo left the clean point: the steps it brings back return with the
    /// flag they had.
    pub(super) fn restore(&self) {
        let mut state = self.state();
        state.live |= state.parked;
        state.parked = false;
    }

    /// Forgets everything: the session closed.
    pub(super) fn reset(&self) {
        *self.state() = State::default();
    }
}
