//! The Restore page's view model: the list of one game's backups, the
//! selection and the flags a restore needs.
//!
//! The restore itself lives in `run.rs`. It copies a backup's bytes over the
//! save folder, backs up the files it replaces first and opens no format
//! (`docs/architecture.md#save-safety`, restore bullets). Nothing here holds
//! a path outside Rust, and `Debug` prints none. A failure reaches the page
//! as a plain outcome, never as error text (`docs/architecture.md#errors`).

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use neoffse::Registry;
use neoffse::core::Game;
use neoffse::io::backup::Backup;

use super::rows::backup_rows;
use super::target::{replaces_open_file, target_of};
use crate::home::title_of;
use crate::state::AppState;
use crate::{BackupRow, RestoreNotice, RestoreProblem};

/// The process scan a restore runs under `Running::Refuse`: the executable
/// names of the game in, the names found running out.
pub(super) type Probe = Arc<dyn Fn(&[&'static str]) -> Vec<&'static str> + Send + Sync>;

/// What the last listing found.
#[derive(Default)]
pub(super) struct Listed {
    pub(super) game: Option<Game>,
    pub(super) backups: Vec<Backup>,
    pub(super) unreadable: usize,
}

pub(super) struct Shared {
    pub(super) registry: Arc<Registry>,
    pub(super) state: Arc<AppState>,
    pub(super) listed: Mutex<Listed>,
    /// Index into `listed.backups` of the row the user chose.
    pub(super) selected: Mutex<Option<usize>>,
    /// Set from the call of `restore` to its `done`.
    pub(super) busy: AtomicBool,
    pub(super) probe: Mutex<Probe>,
}

/// The Restore page over one game's backups.
///
/// Clones share one state. The type is `Send + Sync`. `Debug` prints the game
/// id, the backup count and the busy flag, never a path.
#[derive(Clone)]
pub struct RestoreViewModel {
    pub(super) shared: Arc<Shared>,
}

impl fmt::Debug for RestoreViewModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let listed = self.listed();
        f.debug_struct("RestoreViewModel")
            .field("game", &listed.game.map(Game::id))
            .field("backups", &listed.backups.len())
            .field("busy", &self.is_busy())
            .finish()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl RestoreViewModel {
    /// A page over `state`, with no game listed. The process probe is
    /// `neoffse::io::process::running`.
    #[must_use]
    pub fn new(registry: Arc<Registry>, state: Arc<AppState>) -> Self {
        Self {
            shared: Arc::new(Shared {
                registry,
                state,
                listed: Mutex::new(Listed::default()),
                selected: Mutex::new(None),
                busy: AtomicBool::new(false),
                probe: Mutex::new(Arc::new(neoffse::io::process::running)),
            }),
        }
    }

    pub(super) fn listed(&self) -> MutexGuard<'_, Listed> {
        lock(&self.shared.listed)
    }

    pub(super) fn selection(&self) -> MutexGuard<'_, Option<usize>> {
        lock(&self.shared.selected)
    }

    /// Lists the backups of `game`, newest first, keeps them and clears the
    /// selection.
    ///
    /// Answers `Some(listing-failed)` when the state has no data folder or the
    /// folder cannot be listed; the list is then empty. The call reads at most
    /// 21 small manifests, so it runs on the calling thread.
    #[must_use]
    pub fn show(&self, game: Game) -> Option<RestoreNotice> {
        let listing = self.shared.state.data_dir().and_then(|data| self.shared.registry.backups(&data, game).ok());
        let (listed, notice) = match listing {
            Some(listing) => (
                Listed {
                    game: Some(game),
                    backups: listing.backups,
                    unreadable: listing.unreadable.len(),
                },
                None,
            ),
            None => (
                Listed {
                    game: Some(game),
                    ..Listed::default()
                },
                Some(RestoreNotice {
                    problem: RestoreProblem::ListingFailed,
                    value: "".into(),
                }),
            ),
        };
        *self.listed() = listed;
        *self.selection() = None;
        notice
    }

    /// The game of the last listing, `None` before the first `show` and after
    /// `close`.
    #[must_use]
    pub fn game(&self) -> Option<Game> {
        self.listed().game
    }

    /// The title of the listed game ("Final Fantasy I"), empty before `show`.
    #[must_use]
    pub fn title(&self) -> String {
        self.game().map(title_of).unwrap_or_default()
    }

    /// The rows of the last listing; a row's key is its index.
    #[must_use]
    pub fn rows(&self) -> Vec<BackupRow> {
        backup_rows(&self.listed().backups)
    }

    /// How many backup folders the last listing could not read.
    #[must_use]
    pub fn unreadable_count(&self) -> usize {
        self.listed().unreadable
    }

    /// Remembers the row with `key` and returns it. A key outside the list
    /// clears the selection and returns `None`.
    #[must_use]
    pub fn select(&self, key: i32) -> Option<BackupRow> {
        let rows = self.rows();
        let index = usize::try_from(key).ok().filter(|index| *index < rows.len());
        *self.selection() = index;
        index.and_then(|index| rows.into_iter().nth(index))
    }

    /// The row chosen by `select`.
    #[must_use]
    pub fn selected(&self) -> Option<BackupRow> {
        let index = (*self.selection())?;
        self.rows().into_iter().nth(index)
    }

    /// Whether the selected backup replaces the file the open session holds.
    ///
    /// False with no selection or no open session. The page is unreachable
    /// while a save runs, so the session lock does not wait for a worker.
    #[must_use]
    pub fn replaces_open_save(&self) -> bool {
        let Some(index) = *self.selection() else { return false };
        let Some(session) = self.shared.state.session() else { return false };
        let backup = self.listed().backups.get(index).cloned();
        let Some(backup) = backup else { return false };
        let Some(target) = target_of(&backup.manifest) else { return false };
        let session = lock(&session);
        replaces_open_file(session.dir(), session.name(), &target, &backup.manifest.files)
    }

    /// Whether a restore runs.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.shared.busy.load(Ordering::SeqCst)
    }

    /// Replaces the process probe a restore runs under `Running::Refuse`. The
    /// default is `neoffse::io::process::running`. For tests that must not
    /// depend on the machine's processes.
    pub fn set_process_probe(&self, probe: impl Fn(&[&'static str]) -> Vec<&'static str> + Send + Sync + 'static) {
        *lock(&self.shared.probe) = Arc::new(probe);
    }

    /// Forgets the list and the selection.
    pub fn close(&self) {
        *self.listed() = Listed::default();
        *self.selection() = None;
    }
}
