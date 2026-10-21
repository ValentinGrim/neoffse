//! What the app holds across screens: the open save, and the editor page's
//! view model.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use neoffse::Session;
use neoffse::core::Game;

use crate::SummaryRow;

struct Opened {
    game: Game,
    session: Arc<Mutex<Session>>,
    summary: SummaryRow,
}

/// What the app holds across screens: the open save. The type is
/// `Send + Sync`.
///
/// `Debug` prints whether a save is open and its game id, never a name or a
/// path.
#[derive(Default)]
pub struct AppState {
    opened: Mutex<Option<Opened>>,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let opened = self.lock();
        f.debug_struct("AppState")
            .field("open", &opened.is_some())
            .field("game", &opened.as_ref().map(|opened| opened.game.id()))
            .finish()
    }
}

impl AppState {
    /// A state with no save open.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> MutexGuard<'_, Option<Opened>> {
        self.opened.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The open session, shared with workers. `None` when no save is open.
    #[must_use]
    pub fn session(&self) -> Option<Arc<Mutex<Session>>> {
        self.lock().as_ref().map(|opened| Arc::clone(&opened.session))
    }

    /// Whether a save is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.lock().is_some()
    }

    /// The summary of the open save, as the editor page shows it.
    #[must_use]
    pub fn summary(&self) -> Option<SummaryRow> {
        self.lock().as_ref().map(|opened| opened.summary.clone())
    }

    /// Drops the session without a question. The question for a dirty
    /// session lives in `EditorViewModel::leave`.
    pub fn close(&self) {
        // The old session drops after the lock is released.
        let old = self.lock().take();
        drop(old);
    }

    /// Replaces the open save.
    pub(crate) fn set(&self, session: Session, summary: SummaryRow) {
        let opened = Opened {
            game: session.game(),
            session: Arc::new(Mutex::new(session)),
            summary,
        };
        let old = self.lock().replace(opened);
        drop(old);
    }
}
