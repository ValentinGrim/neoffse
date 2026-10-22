//! What the app holds across screens: the open save, and the editor page's
//! view model.

use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use neoffse::Session;
use neoffse::core::Game;
use neoffse::io::dirs::DataDir;

use crate::SummaryRow;

struct Opened {
    game: Game,
    session: Arc<Mutex<Session>>,
    summary: SummaryRow,
}

/// What the app holds across screens: the open save and the data folder
/// the backups go to. The type is `Send + Sync`.
///
/// `Debug` prints whether a save is open and its game id, never a name or a
/// path.
#[derive(Default)]
pub struct AppState {
    opened: Mutex<Option<Opened>>,
    data_dir: Mutex<Option<DataDir>>,
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

    /// The game of the open save, `None` when none is open.
    #[must_use]
    pub fn game(&self) -> Option<Game> {
        self.lock().as_ref().map(|opened| opened.game)
    }

    /// The summary of the open save, as the editor page shows it.
    #[must_use]
    pub fn summary(&self) -> Option<SummaryRow> {
        self.lock().as_ref().map(|opened| opened.summary.clone())
    }

    /// The data folder a save writes its backup into. `None` until
    /// [`set_data_dir`](Self::set_data_dir): only `run` sets it from the
    /// platform folder, so a state built by a test never touches that folder
    /// and its saves fail.
    #[must_use]
    pub fn data_dir(&self) -> Option<DataDir> {
        self.data_dir.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Sets the data folder for the backups.
    pub fn set_data_dir(&self, data: DataDir) {
        *self.data_dir.lock().unwrap_or_else(PoisonError::into_inner) = Some(data);
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
