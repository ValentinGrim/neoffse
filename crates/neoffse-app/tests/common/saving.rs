//! An open slot wired for saves: a temporary data folder, a probe that finds
//! no game and a log of the save outcomes with the thread each one ran on.
//! Every save writes into the temporary folder.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};

use neoffse::Running;
use neoffse::io::dirs::DataDir;
use neoffse_app::editor::SaveOutcome;
use tempfile::TempDir;

use super::{Editing, ID_1, Site, backup_count, break_data_dir, editing, gil_in, slot_file, steam_folder};

type Log = Arc<Mutex<Vec<(SaveOutcome, ThreadId)>>>;

pub(crate) struct Saving {
    pub(crate) ed: Editing,
    pub(crate) data: TempDir,
    pub(crate) file: PathBuf,
    log: Log,
}

/// An open slot, a temporary data folder and a probe that finds no game.
pub(crate) fn saving(site: &Site) -> Saving {
    let ed = editing(site);
    let data = tempfile::tempdir().unwrap();
    ed.flow.state.set_data_dir(DataDir::at(data.path()));
    ed.editor().set_process_probe(|_| Vec::new());
    let file = steam_folder(site, 0, ID_1).join(slot_file(1));
    Saving {
        ed,
        data,
        file,
        log: Log::default(),
    }
}

impl Saving {
    pub(crate) fn edit_gil(&self, value: &str) {
        self.ed.set_gil(value);
    }

    /// Starts a save; its outcome lands in the log when the dispatcher runs.
    pub(crate) fn save(&self, running: Running) {
        let log = Arc::clone(&self.log);
        self.ed.editor().save(running, &self.ed.flow.manual.dispatcher(), move |outcome| {
            log.lock().unwrap_or_else(PoisonError::into_inner).push((outcome, thread::current().id()));
        });
    }

    pub(crate) fn outcomes(&self) -> Vec<SaveOutcome> {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(|(outcome, _)| outcome.clone())
            .collect()
    }

    /// The thread each outcome was reported on, in order.
    pub(crate) fn threads(&self) -> Vec<ThreadId> {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(|(_, thread)| *thread)
            .collect()
    }

    pub(crate) fn backups(&self) -> usize {
        backup_count(self.data.path())
    }

    pub(crate) fn gil_on_disk(&self) -> i64 {
        gil_in(&self.file)
    }

    /// Points the data folder at a plain file.
    pub(crate) fn break_data_dir(&self) {
        break_data_dir(&self.ed.flow.state, self.data.path());
    }
}
