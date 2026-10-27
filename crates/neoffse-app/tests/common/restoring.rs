//! The Restore view model over one synthetic FF I slot: a temporary data
//! folder, the open flow on the same state and a scripted process probe.
//! Nothing here names the platform data folder or the real process list.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};

use i_slint_backend_testing::AccessibleRole::Button;
use neoffse::Running;
use neoffse::core::Game;
use neoffse::io::backup::Backup;
use neoffse::io::dirs::DataDir;
use neoffse_app::home::OpenOutcome;
use neoffse_app::{BackupRow, RestoreBridge, RestoreOutcome, RestoreViewModel};
use slint::{ComponentHandle, Model};
use tempfile::TempDir;

use super::{Flow, ID_1, Outcomes, Site, T0, Ui, editing_site, items, listing, make_backup, open_editor, scripted_probe, slot_bytes, slot_file, steam_folder};

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
        vm.set_process_probe(scripted_probe(counter, runs));
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

    /// A backup of slot 1 as `slot_bytes(1)`, then the folder holds `slot_bytes(2)`.
    pub(crate) fn differing() -> Self {
        let r = Self::new();
        r.backup_now("before-save", 0);
        fs::write(r.file(), slot_bytes(2)).unwrap();
        r
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

/// A window wired for the Restore page, over a temporary data folder. It
/// holds the save folder of FF I, where slot 1 lives.
pub(crate) struct Paged {
    pub(crate) ui: Ui,
    pub(crate) data: TempDir,
    pub(crate) folder: PathBuf,
    /// How many times the restore's process probe ran.
    pub(crate) probes: Arc<AtomicUsize>,
    game_runs: Arc<AtomicBool>,
}

impl Paged {
    /// Home, scanned to the end, nothing open.
    pub(crate) fn home(site: &Site) -> Self {
        Self::over(Ui::scanned(site, None), site)
    }

    /// The editor open on slot 1, a save probe that finds no game.
    pub(crate) fn editing(site: &Site) -> Self {
        let p = Self::over(open_editor(site), site);
        p.ui.app.editor.set_process_probe(|_| Vec::new());
        p
    }

    /// `ui` over a temporary data folder and a scripted restore probe that
    /// finds no game.
    pub(crate) fn over(ui: Ui, site: &Site) -> Self {
        let data = tempfile::tempdir().unwrap();
        ui.app.state.set_data_dir(DataDir::at(data.path()));
        let probes = Arc::new(AtomicUsize::new(0));
        let game_runs = Arc::new(AtomicBool::new(false));
        let (counter, runs) = (Arc::clone(&probes), Arc::clone(&game_runs));
        ui.app.restore.set_process_probe(scripted_probe(counter, runs));
        Self {
            ui,
            data,
            folder: steam_folder(site, 0, ID_1),
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

    /// Backs up slot 1 as it is now, at `T0` plus `minutes`, UTC.
    pub(crate) fn backup(&self, reason: &str, minutes: i64) -> Backup {
        make_backup(&self.data_dir(), "ff1-pr", &self.folder, &[slot_file(1)], reason, T0 + minutes * 60, 0)
    }

    pub(crate) fn bridge(&self) -> RestoreBridge<'_> {
        self.ui.window.global::<RestoreBridge>()
    }

    /// The rows the page shows.
    pub(crate) fn rows(&self) -> Vec<BackupRow> {
        items(&self.bridge().get_rows())
    }

    /// The stamps of the rows, in order.
    pub(crate) fn stamps(&self) -> Vec<String> {
        self.rows().iter().map(|row| row.stamp.to_string()).collect()
    }

    /// The bytes of slot 1 in the save folder.
    pub(crate) fn file(&self) -> Vec<u8> {
        fs::read(self.folder.join(slot_file(1))).unwrap()
    }

    /// Clicks the row of the backup at `T0`, then Restore in the confirmation.
    /// The worker has not run.
    pub(crate) fn start(&self) {
        self.ui
            .element("Restore the backup of 2020-09-13 12:26 UTC", Button)
            .invoke_accessible_default_action();
        assert!(self.bridge().get_confirm_open());
        self.ui.element("Restore", Button).invoke_accessible_default_action();
    }

    /// Opens the page of FF I from Home.
    pub(crate) fn open_from_home(&self) {
        self.ui.window.invoke_backups_requested("ff1-pr".into());
        assert!(self.ui.window.get_restore_open());
    }

    /// Opens the page from the File menu with the editor open.
    pub(crate) fn open_from_menu(&self) {
        self.ui.window.invoke_restore_menu_requested();
        assert!(self.ui.window.get_restore_open());
    }
}

/// Home with the page of FF I open. The one backup, made at `T0`, holds
/// `slot_bytes(1)`; the folder holds `slot_bytes(2)`. The site must outlive
/// the page.
pub(crate) fn restore_page() -> (Site, Paged) {
    let site = editing_site();
    let p = Paged::home(&site);
    p.backup("before-save", 0);
    fs::write(p.folder.join(slot_file(1)), slot_bytes(2)).unwrap();
    p.open_from_home();
    (site, p)
}
