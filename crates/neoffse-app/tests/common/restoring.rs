//! The Restore view model over one synthetic FF I slot: a temporary data
//! folder, the open flow on the same state and a scripted process probe.
//! Nothing here names the platform data folder or the real process list.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};

use neoffse::Running;
use neoffse::core::Game;
use neoffse::io::backup::Backup;
use neoffse::io::dirs::DataDir;
use neoffse::pr::games::GAMES;
use neoffse_app::home::OpenOutcome;
use neoffse_app::{RestoreOutcome, RestoreViewModel};
use tempfile::TempDir;

use super::{Flow, ID_1, Outcomes, Site, T0, editing_site, listing, make_backup, slot_file, steam_folder};

/// Where `done` ran, and with what.
#[derive(Clone, Default)]
pub(crate) struct Ended {
    log: Arc<Mutex<Vec<(RestoreOutcome, ThreadId)>>>,
}

impl Ended {
    /// A `done` closure that appends to this log.
    pub(crate) fn recorder(&self) -> impl FnOnce(RestoreOutcome) + Send + 'static {
        let log = Arc::clone(&self.log);
        move |outcome| log.lock().unwrap_or_else(PoisonError::into_inner).push((outcome, thread::current().id()))
    }

    pub(crate) fn all(&self) -> Vec<(RestoreOutcome, ThreadId)> {
        self.log.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// The outcomes alone, in order.
    pub(crate) fn outcomes(&self) -> Vec<RestoreOutcome> {
        self.all().into_iter().map(|(outcome, _)| outcome).collect()
    }
}

pub(crate) struct Restoring {
    pub(crate) site: Site,
    pub(crate) flow: Flow,
    pub(crate) vm: RestoreViewModel,
    pub(crate) data: TempDir,
    /// The save folder of FF I; slot 1 holds `slot_bytes(1)` at the start.
    pub(crate) folder: PathBuf,
    /// How many times the probe ran.
    pub(crate) probes: Arc<AtomicUsize>,
    game_runs: Arc<AtomicBool>,
}

impl Restoring {
    /// A temporary machine with slot 1, a temporary data folder, a probe that
    /// finds no game.
    pub(crate) fn new() -> Self {
        let site = editing_site();
        let flow = Flow::new(&site, None);
        let data = tempfile::tempdir().unwrap();
        flow.state.set_data_dir(DataDir::at(data.path()));
        let vm = RestoreViewModel::new(super::registry(), Arc::clone(&flow.state));
        let probes = Arc::new(AtomicUsize::new(0));
        let game_runs = Arc::new(AtomicBool::new(false));
        let (counter, runs) = (Arc::clone(&probes), Arc::clone(&game_runs));
        vm.set_process_probe(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
            if runs.load(Ordering::SeqCst) { vec![GAMES[0].executable] } else { Vec::new() }
        });
        let folder = steam_folder(&site, 0, ID_1);
        Self {
            site,
            flow,
            vm,
            data,
            folder,
            probes,
            game_runs,
        }
    }

    /// The probe finds the FF I executable from now on.
    pub(crate) fn game_runs(&self, runs: bool) {
        self.game_runs.store(runs, Ordering::SeqCst);
    }

    pub(crate) fn data_dir(&self) -> DataDir {
        DataDir::at(self.data.path())
    }

    pub(crate) fn file(&self) -> PathBuf {
        self.folder.join(slot_file(1))
    }

    pub(crate) fn bytes(&self) -> Vec<u8> {
        fs::read(self.file()).unwrap()
    }

    pub(crate) fn backups(&self, game: Game) -> Vec<Backup> {
        listing(&self.data_dir(), game)
    }

    /// The reasons of the FF I backups, newest first.
    pub(crate) fn reasons(&self) -> Vec<String> {
        self.backups(Game::Ff1Pr).into_iter().map(|backup| backup.manifest.reason).collect()
    }

    /// Backs up slot 1 as it is now, at `T0` plus `minutes`, UTC.
    pub(crate) fn backup_now(&self, reason: &str, minutes: i64) -> Backup {
        make_backup(&self.data_dir(), "ff1-pr", &self.folder, &[slot_file(1)], reason, T0 + minutes * 60, 0)
    }

    /// Opens slot 1 in the shared state, run to the end.
    pub(crate) fn open_slot(&self) {
        let outcomes = Outcomes::new();
        self.flow.click(slot_file(1), &outcomes);
        self.flow.manual.run_all();
        assert_eq!(outcomes.outcomes(), vec![OpenOutcome::Opened]);
    }

    /// Restores the selection and runs every queued step. Returns the one
    /// outcome.
    pub(crate) fn restore(&self, running: Running) -> RestoreOutcome {
        let ended = Ended::default();
        self.vm.restore(running, &self.flow.manual.dispatcher(), ended.recorder());
        self.flow.manual.run_all();
        let mut outcomes = ended.outcomes();
        assert_eq!(outcomes.len(), 1, "one outcome per restore call");
        outcomes.remove(0)
    }

    /// Lists FF I and selects row `key`.
    pub(crate) fn pick(&self, key: i32) {
        assert!(self.vm.show(Game::Ff1Pr).is_none());
        assert!(self.vm.select(key).is_some());
    }
}
