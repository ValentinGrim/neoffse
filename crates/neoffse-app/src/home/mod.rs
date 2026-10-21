//! The Home screen's view model: the saves discovery finds, grouped game,
//! folder, slot, and the filter that hides the games the user does not own.
//!
//! [`scan_home`] does the blocking work and returns plain data.
//! [`HomeViewModel`] keeps that data, the filter and the scan flag, and turns
//! them into the Slint rows on the UI thread. Nothing here needs a window.

mod rows;
mod scan;

use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use neoffse::Registry;
use neoffse::core::Game;
use neoffse::io::discovery::Environment;

use crate::dispatch::{Dispatcher, run_async};
use crate::{FolderOrigin, GameRow, SlotKind};

pub use scan::scan_home;

/// What one scan found. Plain data, `Send`: it crosses from the worker to the
/// UI thread.
///
/// `Debug` prints save content (party names, locations) and paths: never log
/// it.
#[derive(Debug, Clone)]
pub struct HomeData {
    games: Vec<GameData>,
    skipped: usize,
}

#[derive(Debug, Clone)]
struct GameData {
    game: Game,
    supported: bool,
    owned: bool,
    folders: Vec<FolderData>,
}

#[derive(Debug, Clone)]
struct FolderData {
    origin: FolderOrigin,
    dir: PathBuf,
    slots: Vec<SlotData>,
}

#[derive(Debug, Clone)]
struct SlotData {
    key: i32,
    name: String,
    kind: SlotKind,
    number: i32,
    summary: Option<SummaryData>,
    unreadable: bool,
    openable: bool,
}

#[derive(Debug, Clone)]
struct SummaryData {
    party: String,
    location: String,
    play_time: String,
}

/// Where a slot sits: the game, the folder and the file name.
///
/// Built by [`HomeViewModel::slot_target`] only. `Debug` prints the game id
/// and the length of the name, never a path or a name.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SlotTarget {
    /// The game of the folder.
    pub game: Game,
    /// The folder that holds the file.
    pub dir: PathBuf,
    /// The file name inside `dir`.
    pub name: String,
    /// Whether a format can open the file.
    pub supported: bool,
}

impl fmt::Debug for SlotTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SlotTarget")
            .field("game", &self.game.id())
            .field("name_len", &self.name.len())
            .field("supported", &self.supported)
            .finish_non_exhaustive()
    }
}

struct State {
    data: HomeData,
    owned_only: bool,
    scanning: bool,
}

struct Shared {
    registry: Arc<Registry>,
    environment: Environment,
    state: Mutex<State>,
}

/// The Home screen's state: the last scan, the filter and the scan flag.
///
/// Clones share one state. The type is `Send + Sync`.
#[derive(Clone)]
pub struct HomeViewModel {
    shared: Arc<Shared>,
}

impl fmt::Debug for HomeViewModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = self.lock();
        f.debug_struct("HomeViewModel")
            .field("scanning", &state.scanning)
            .field("owned_only", &state.owned_only)
            .field("games", &state.data.games.len())
            .finish()
    }
}

impl HomeViewModel {
    /// A view model with the owned-only filter on, nothing scanned and no
    /// scan running.
    #[must_use]
    pub fn new(registry: Arc<Registry>, environment: Environment) -> Self {
        let state = State {
            data: HomeData { games: Vec::new(), skipped: 0 },
            owned_only: true,
            scanning: false,
        };
        Self {
            shared: Arc::new(Shared {
                registry,
                environment,
                state: Mutex::new(state),
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.shared.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts a scan through [`run_async`]. `done` runs on the UI thread
    /// after the new data is in place. When a scan already runs, nothing
    /// starts and `done` is dropped.
    pub fn start_scan(&self, dispatcher: &Arc<dyn Dispatcher>, done: impl FnOnce() + Send + 'static) {
        {
            let mut state = self.lock();
            if state.scanning {
                return;
            }
            state.scanning = true;
        }
        let worker = Arc::clone(&self.shared);
        let target = self.clone();
        run_async(
            dispatcher,
            move || scan_home(&worker.registry, &worker.environment),
            move |data| {
                target.finish_scan(data);
                done();
            },
        );
    }

    fn finish_scan(&self, data: HomeData) {
        let mut state = self.lock();
        state.data = data;
        state.scanning = false;
    }

    /// Whether a scan is running.
    #[must_use]
    pub fn is_scanning(&self) -> bool {
        self.lock().scanning
    }

    /// Whether the filter hides the games with no install and no save folder.
    #[must_use]
    pub fn owned_only(&self) -> bool {
        self.lock().owned_only
    }

    /// Turns the owned-only filter on or off. The choice is not stored on
    /// disk.
    pub fn set_owned_only(&self, owned_only: bool) {
        self.lock().owned_only = owned_only;
    }

    /// The visible games, filter applied. Empty before the first scan
    /// finishes. Call it on the UI thread: the rows hold Slint models.
    #[must_use]
    pub fn rows(&self) -> Vec<GameRow> {
        let state = self.lock();
        rows::game_rows(&state.data, state.owned_only)
    }

    /// The places discovery or the folder scans could not read, plus the file
    /// names that are not UTF-8.
    #[must_use]
    pub fn skipped_count(&self) -> usize {
        self.lock().data.skipped
    }

    /// The slot behind a `SlotRow::key` of the current data. `None` for a key
    /// the data does not hold.
    #[must_use]
    pub fn slot_target(&self, key: i32) -> Option<SlotTarget> {
        let state = self.lock();
        state.data.games.iter().find_map(|game| {
            game.folders.iter().find_map(|folder| {
                let slot = folder.slots.iter().find(|slot| slot.key == key)?;
                Some(SlotTarget {
                    game: game.game,
                    dir: folder.dir.clone(),
                    name: slot.name.clone(),
                    supported: slot.openable,
                })
            })
        })
    }
}
