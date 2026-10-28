//! A window over the first synthetic slot, wired for saves: a temporary data
//! folder and process probes that count their calls. Nothing here names the
//! platform data folder or the real process list.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use i_slint_backend_testing::AccessibleRole::Button;
use neoffse::core::{Capability, Edit, FieldId, Game, Value};
use neoffse::data::Language;
use neoffse::io::backup::Backup;
use neoffse::io::dirs::DataDir;
use neoffse::pr::games::GAMES;
use neoffse::{Registry, Running, Session};
use neoffse_app::AboutDialog;
use neoffse_app::state::AppState;
use slint::ComponentHandle;
use tempfile::TempDir;

use super::{ID_1, Site, T0, Ui, make_backup, open_editor, slot_file, steam_folder};

/// What the scripted probe answers.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Machine {
    /// No process runs.
    Quiet,
    /// The FF I executable runs.
    GameRuns,
}

pub(crate) struct Rig {
    pub(crate) ui: Ui,
    pub(crate) data: TempDir,
    pub(crate) file: PathBuf,
    pub(crate) probes: Arc<AtomicUsize>,
}

/// A process probe that counts its calls in `counter` and finds the FF I
/// executable while `runs` is set.
pub(crate) fn scripted_probe(counter: Arc<AtomicUsize>, runs: Arc<AtomicBool>) -> impl Fn(&[&'static str]) -> Vec<&'static str> + Send + Sync + 'static {
    move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
        if runs.load(Ordering::SeqCst) { vec![GAMES[0].executable] } else { Vec::new() }
    }
}

/// Sets the save probe and the restore probe of `ui` to answer `machine`.
/// Both count into the returned counter.
fn scripted_probes(ui: &Ui, machine: Machine) -> Arc<AtomicUsize> {
    let probes = Arc::new(AtomicUsize::new(0));
    let runs = Arc::new(AtomicBool::new(machine == Machine::GameRuns));
    ui.app.editor.set_process_probe(scripted_probe(Arc::clone(&probes), Arc::clone(&runs)));
    ui.app.restore.set_process_probe(scripted_probe(Arc::clone(&probes), runs));
    probes
}

/// Points the data folder of `state` at a plain file inside `folder`: no
/// backup, listing or save can use it.
pub(crate) fn break_data_dir(state: &AppState, folder: &Path) {
    let not_a_folder = folder.join("not-a-folder");
    fs::write(&not_a_folder, b"x").unwrap();
    state.set_data_dir(DataDir::at(&not_a_folder));
}

/// The `created` text of the newest FF I backup under the data folder `data`.
pub(crate) fn newest_created(data: &Path) -> String {
    let mut listing = Registry::new(Language::En).backups(&DataDir::at(data), Game::Ff1Pr).unwrap();
    listing.backups.remove(0).manifest.created
}

/// How many FF I backups the data folder `data` holds.
pub(crate) fn backup_count(data: &Path) -> usize {
    Registry::new(Language::En)
        .backups(&DataDir::at(data), Game::Ff1Pr)
        .map_or(0, |listing| listing.backups.len())
}

fn session_in(file: &Path) -> Session {
    Registry::new(Language::En)
        .open(file.parent().unwrap(), file.file_name().unwrap().to_str().unwrap(), Some(Game::Ff1Pr))
        .unwrap()
}

/// The gil the save file `file` holds.
pub(crate) fn gil_in(file: &Path) -> i64 {
    let view = session_in(file).view().unwrap();
    let entity = view
        .section(Capability::Inventory)
        .unwrap()
        .entities
        .iter()
        .find(|e| e.id == FieldId::new("gil"))
        .unwrap()
        .clone();
    match entity.fields[0].value {
        Value::Int(gil) => gil,
        ref other => panic!("the gil is not an integer: {other:?}"),
    }
}

/// The editor open on slot 1, a temporary data folder set, probes that
/// answer `machine`.
pub(crate) fn rig(site: &Site, machine: Machine) -> Rig {
    let ui = open_editor(site);
    let data = tempfile::tempdir().unwrap();
    ui.app.state.set_data_dir(DataDir::at(data.path()));
    let probes = scripted_probes(&ui, machine);
    let file = steam_folder(site, 0, ID_1).join(slot_file(1));
    Rig { ui, data, file, probes }
}

/// A shown window with About connected and the close handler installed, the
/// first slot open on a temporary data folder and probes that answer
/// `machine`.
pub(crate) fn closeable_rig(site: &Site, machine: Machine) -> (Rig, AboutDialog) {
    let (ui, about) = Ui::with_about(site);
    ui.open_first_slot();
    ui.window.show().unwrap();
    let data = tempfile::tempdir().unwrap();
    ui.app.state.set_data_dir(DataDir::at(data.path()));
    let probes = scripted_probes(&ui, machine);
    let file = steam_folder(site, 0, ID_1).join(slot_file(1));
    (Rig { ui, data, file, probes }, about)
}

/// A save held inside the process probe: its worker owns the session lock until `release`.
pub(crate) struct HeldSave {
    worker: JoinHandle<()>,
    release: mpsc::Sender<()>,
    timed_out: Arc<AtomicBool>,
}

impl HeldSave {
    /// Lets the worker go and joins it. Panics with "a UI callback waited for the
    /// running save" when the probe hit its 10 s timeout, which means a callback on
    /// the UI thread blocked on the session lock. Does not run the posted results:
    /// the caller runs `rig.ui.manual.run_all()` afterwards.
    pub(crate) fn release(self) {
        self.release.send(()).unwrap();
        self.worker.join().unwrap();
        assert!(!self.timed_out.load(Ordering::SeqCst), "a UI callback waited for the running save");
    }
}

impl Rig {
    pub(crate) fn data_dir(&self) -> DataDir {
        DataDir::at(self.data.path())
    }

    /// Points the data folder at a plain file.
    pub(crate) fn break_data_dir(&self) {
        break_data_dir(&self.ui.app.state, self.data.path());
    }

    /// Opens the Restore page of the open game from the File menu.
    pub(crate) fn open_page(&self) {
        self.ui.window.invoke_restore_menu_requested();
        assert!(self.ui.window.get_restore_open());
    }

    /// Clicks the row of the backup at `T0`: the confirmation opens.
    pub(crate) fn pick_backup(&self) {
        self.ui
            .element("Restore the backup of 2020-09-13 12:26 UTC", Button)
            .invoke_accessible_default_action();
        assert!(self.ui.restore_bridge().get_confirm_open());
    }

    /// Picks the backup at `T0`, then clicks Restore in the confirmation.
    /// Nothing has run: the next step is the worker, or the discard dialog.
    pub(crate) fn start_restore(&self) {
        self.pick_backup();
        self.ui.element("Restore", Button).invoke_accessible_default_action();
    }

    /// Installs a probe that signals, then waits up to 10 s for `release`; calls
    /// `invoke_save_requested`; runs the jobs on a second thread; returns once
    /// the worker is inside the probe. Needs a dirty session.
    pub(crate) fn hold_save(&self) -> HeldSave {
        let (entered_tx, entered) = mpsc::channel::<()>();
        let (release, release_rx) = mpsc::channel::<()>();
        let release_rx = Mutex::new(release_rx);
        let timed_out = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&timed_out);
        // The probe runs inside `Session::save`, under the session lock.
        self.ui.app.editor.set_process_probe(move |_| {
            entered_tx.send(()).unwrap();
            let waited = release_rx.lock().unwrap().recv_timeout(Duration::from_secs(10));
            flag.store(waited.is_err(), Ordering::SeqCst);
            Vec::new()
        });
        self.ui.bridge().invoke_save_requested();
        let manual = Arc::clone(&self.ui.manual);
        let worker = thread::spawn(move || {
            manual.run_jobs();
        });
        entered.recv_timeout(Duration::from_secs(10)).expect("the save reached the probe");
        HeldSave { worker, release, timed_out }
    }

    /// Backs up the rig's file as it is now, at `T0`, UTC.
    pub(crate) fn backup_now(&self, reason: &str) -> Backup {
        let folder = self.file.parent().unwrap();
        make_backup(&self.data_dir(), "ff1-pr", folder, &[slot_file(1)], reason, T0, 0)
    }

    /// The `created` text of the newest backup.
    pub(crate) fn newest_created(&self) -> String {
        newest_created(self.data.path())
    }

    pub(crate) fn backups(&self) -> usize {
        backup_count(self.data.path())
    }

    pub(crate) fn bytes(&self) -> Vec<u8> {
        fs::read(&self.file).unwrap()
    }

    /// Another program saves gil `value` into the file: a second session.
    pub(crate) fn change_on_disk(&self, value: i64) {
        let mut other = session_in(&self.file);
        other.edit(&Edit::set(FieldId::new("gil"), Value::Int(value))).unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        other.save(&DataDir::at(elsewhere.path()), Running::Allow).unwrap();
    }

    /// The gil the file holds.
    pub(crate) fn gil_on_disk(&self) -> i64 {
        gil_in(&self.file)
    }
}
