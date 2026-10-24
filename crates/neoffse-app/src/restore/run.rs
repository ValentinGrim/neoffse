//! The restore on a worker thread, and the answer the page needs.
//!
//! The pipeline (safety backup, verified atomic writes) lives in
//! `Registry::restore_with` (`docs/architecture.md#save-safety`, restore
//! bullets). This module checks the target folder, picks the data folder, the
//! probe and the thread, and turns the result into a [`RestoreOutcome`]. No
//! error text leaves it (`docs/architecture.md#errors`).

use std::sync::atomic::Ordering;
use std::sync::{Arc, PoisonError};

use neoffse::io::atomic::RealFileOps;
use neoffse::io::backup::{Moment, Restored};
use neoffse::io::error::IoError;
use neoffse::{Error, Running};

use super::model::RestoreViewModel;
use super::target::target_of;
use crate::dispatch::{Dispatcher, run_async};
use crate::editor::stamp_of;

/// How a restore ended.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RestoreOutcome {
    /// Every file is written. `safety` is the date of the safety backup
    /// ("2026-10-22 14:05 UTC"), `None` when no file existed to back up or
    /// the date did not parse. `replaced_open` says the restore replaced the
    /// file the open session holds.
    Restored {
        /// The date of the safety backup for the notice.
        safety: Option<String>,
        /// The restore replaced the open session's file.
        replaced_open: bool,
    },
    /// The process scan found the game. Nothing was read or written.
    GameRunning,
    /// The save folder of the backup does not exist, or the manifest names no
    /// usable folder. Nothing was read or written.
    FolderMissing,
    /// An error before the first write: no file changed. Also no selection
    /// and no data folder.
    Failed,
    /// The restore stopped after the first write, or the worker panicked.
    /// Some files may be replaced.
    Incomplete,
    /// Another restore runs. This call did nothing.
    Busy,
}

/// The outcome of a finished pipeline.
fn outcome_of(result: Result<Restored, Error>, replaced_open: bool) -> RestoreOutcome {
    match result {
        Ok(restored) => RestoreOutcome::Restored {
            safety: restored.safety.and_then(|backup| stamp_of(&backup.manifest.created)),
            replaced_open,
        },
        Err(Error::GameRunning { .. }) => RestoreOutcome::GameRunning,
        Err(Error::Io {
            source: IoError::RestoreFailed { .. },
        }) => RestoreOutcome::Incomplete,
        Err(_) => RestoreOutcome::Failed,
    }
}

impl RestoreViewModel {
    /// Restores the selected backup on a worker thread.
    ///
    /// Four cases answer through `done` at once and queue no job: no
    /// selection (`Failed`), a manifest with no absolute folder
    /// (`FolderMissing`), no data folder in the state (`Failed`) and a restore
    /// already running (`Busy`, the flag untouched). Otherwise the worker
    /// answers `FolderMissing` when the folder is gone, before any probe or
    /// write, and else runs `Registry::restore_with` with the real file
    /// operations, the process probe and the current time. A worker panic
    /// gives `Incomplete`; the files may be half written by then.
    ///
    /// `done` runs once on the UI thread, with [`is_busy`](Self::is_busy)
    /// already false. The selection clears on `Restored` only, so the
    /// running-game warning can call `restore` again on the same row. Neither
    /// `select` nor `restore` refreshes the list; the caller calls
    /// [`show`](Self::show) again.
    pub fn restore(&self, running: Running, dispatcher: &Arc<dyn Dispatcher>, done: impl FnOnce(RestoreOutcome) + Send + 'static) {
        let Some(index) = *self.selection() else {
            done(RestoreOutcome::Failed);
            return;
        };
        let replaced_open = self.replaces_open_save();
        let backup = self.listed().backups.get(index).cloned();
        let Some(backup) = backup else {
            done(RestoreOutcome::Failed);
            return;
        };
        let Some(target) = target_of(&backup.manifest) else {
            done(RestoreOutcome::FolderMissing);
            return;
        };
        let Some(data) = self.shared.state.data_dir() else {
            done(RestoreOutcome::Failed);
            return;
        };
        if self.shared.busy.swap(true, Ordering::SeqCst) {
            done(RestoreOutcome::Busy);
            return;
        }
        let probe = Arc::clone(&self.shared.probe.lock().unwrap_or_else(PoisonError::into_inner));
        let registry = Arc::clone(&self.shared.registry);
        let work = move || {
            if !target.is_dir() {
                return RestoreOutcome::FolderMissing;
            }
            let result = registry.restore_with(&RealFileOps, |names| probe(names), Moment::now(), &data, &backup, &target, running);
            outcome_of(result, replaced_open)
        };
        let shared = Arc::clone(&self.shared);
        run_async(dispatcher, work, move |result| {
            shared.busy.store(false, Ordering::SeqCst);
            let outcome = result.unwrap_or(RestoreOutcome::Incomplete);
            if matches!(outcome, RestoreOutcome::Restored { .. }) {
                *shared.selected.lock().unwrap_or_else(PoisonError::into_inner) = None;
            }
            done(outcome);
        });
    }
}
