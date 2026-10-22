//! The save: the facade's pipeline on a worker thread, and the answer the
//! screen needs.
//!
//! The pipeline itself (backup, atomic write, checks) lives in
//! `Session::save` (`docs/architecture.md#save-safety`). This module only
//! picks the data folder, the process probe and the thread, and turns the
//! result into a [`SaveOutcome`].

use std::sync::atomic::Ordering;
use std::sync::{Arc, PoisonError};

use neoffse::io::atomic::RealFileOps;
use neoffse::io::backup::Moment;
use neoffse::{Error, Running, Saved};

use super::model::{EditorViewModel, lock_session};
use super::review::stamp_of;
use crate::dispatch::{Dispatcher, run_async};

/// How a save ended.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SaveOutcome {
    /// The file is written and the session is clean. `stamp` is the time of
    /// the backup ("2026-10-22 14:05 UTC"), `None` when nothing was written
    /// (a clean session) or the backup time did not parse.
    Saved {
        /// The backup time for the notice.
        stamp: Option<String>,
    },
    /// The process scan found the game. Nothing was written.
    GameRunning,
    /// The file changed on disk since the session read it. Nothing was
    /// written.
    ChangedOnDisk,
    /// Any other failure, or no save open, or no data folder, or a worker
    /// panic. The session keeps its edits.
    Failed,
    /// Another save is running. This call did nothing.
    Busy,
}

/// The outcome of a finished pipeline. No error text leaves this function
/// (`docs/architecture.md#errors`).
fn outcome_of(result: Result<Saved, Error>) -> SaveOutcome {
    match result {
        Ok(saved) => SaveOutcome::Saved {
            stamp: saved.backup.and_then(|backup| stamp_of(&backup.manifest.created)),
        },
        Err(Error::GameRunning { .. }) => SaveOutcome::GameRunning,
        Err(Error::ChangedOnDisk { .. }) => SaveOutcome::ChangedOnDisk,
        Err(_) => SaveOutcome::Failed,
    }
}

impl EditorViewModel {
    /// Whether a save runs. The worker holds the session lock then, so edits,
    /// undo, redo and the review refuse to run.
    #[must_use]
    pub fn is_saving(&self) -> bool {
        self.shared.saving.load(Ordering::SeqCst)
    }

    /// Saves the open session on a worker thread.
    ///
    /// With no save open, or no data folder in the
    /// [`AppState`](crate::state::AppState), `done(Failed)` runs at once and
    /// no job is queued: an unset data folder never falls back to the
    /// platform folder. A save already running gives `done(Busy)` at once and
    /// leaves the flag alone. Otherwise the worker locks the session and runs
    /// `Session::save_with` with the real file operations, the process probe
    /// and the current time. A worker panic gives `Failed`; a poisoned
    /// session lock is recovered.
    ///
    /// `done` runs once on the UI thread, with [`is_saving`](Self::is_saving)
    /// already false. The expert flag is untouched.
    pub fn save(&self, running: Running, dispatcher: &Arc<dyn Dispatcher>, done: impl FnOnce(SaveOutcome) + Send + 'static) {
        let (Some(session), Some(data)) = (self.shared.state.session(), self.shared.state.data_dir()) else {
            done(SaveOutcome::Failed);
            return;
        };
        if self.shared.saving.swap(true, Ordering::SeqCst) {
            done(SaveOutcome::Busy);
            return;
        }
        let probe = Arc::clone(&self.shared.probe.lock().unwrap_or_else(PoisonError::into_inner));
        let shared = Arc::clone(&self.shared);
        let work = move || {
            let mut session = lock_session(&session);
            outcome_of(session.save_with(&RealFileOps, |names| probe(names), Moment::now(), &data, running))
        };
        run_async(dispatcher, work, move |result| {
            shared.saving.store(false, Ordering::SeqCst);
            done(result.unwrap_or(SaveOutcome::Failed));
        });
    }

    /// Replaces the process probe a save runs under [`Running::Refuse`]. The
    /// default is `neoffse::io::process::running`. For tests that must not
    /// depend on the machine's processes.
    pub fn set_process_probe(&self, probe: impl Fn(&[&'static str]) -> Vec<&'static str> + Send + Sync + 'static) {
        *self.shared.probe.lock().unwrap_or_else(PoisonError::into_inner) = Arc::new(probe);
    }
}
