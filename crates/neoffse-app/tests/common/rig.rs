//! A window over the first synthetic slot, wired for saves: a temporary data
//! folder and a process probe that counts its calls. Nothing here names the
//! platform data folder or the real process list.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use i_slint_backend_testing::AccessibleRole::Button;
use neoffse::core::{Capability, Edit, FieldId, Game, Value};
use neoffse::data::Language;
use neoffse::io::backup::Backup;
use neoffse::io::dirs::DataDir;
use neoffse::pr::games::GAMES;
use neoffse::{Registry, Running, Session};
use neoffse_app::AboutDialog;
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

/// The editor open on slot 1, a temporary data folder set, a probe that
/// answers `machine`.
pub(crate) fn rig(site: &Site, machine: Machine) -> Rig {
    let ui = open_editor(site);
    let data = tempfile::tempdir().unwrap();
    ui.app.state.set_data_dir(DataDir::at(data.path()));
    let probes = Arc::new(AtomicUsize::new(0));
    // One counter for the save probe and the restore probe.
    let (save_counter, restore_counter) = (Arc::clone(&probes), Arc::clone(&probes));
    let scripted = move |counter: &AtomicUsize| {
        counter.fetch_add(1, Ordering::SeqCst);
        if machine == Machine::GameRuns {
            vec![GAMES[0].executable]
        } else {
            Vec::new()
        }
    };
    ui.app.editor.set_process_probe(move |_| scripted(&save_counter));
    ui.app.restore.set_process_probe(move |_| scripted(&restore_counter));
    let file = steam_folder(site, 0, ID_1).join(slot_file(1));
    Rig { ui, data, file, probes }
}

/// A shown window with About connected and the close handler installed, the
/// first slot open on a temporary data folder and a quiet restore probe.
pub(crate) fn closeable_rig(site: &Site) -> (Rig, AboutDialog) {
    let (ui, about) = Ui::with_about(site);
    let key = ui.first_slots()[0].key;
    ui.window.invoke_slot_clicked(key);
    ui.manual.run_all();
    assert!(ui.window.get_editor_open());
    ui.window.show().unwrap();
    let data = tempfile::tempdir().unwrap();
    ui.app.state.set_data_dir(DataDir::at(data.path()));
    ui.app.restore.set_process_probe(|_| Vec::new());
    let file = steam_folder(site, 0, ID_1).join(slot_file(1));
    (
        Rig {
            ui,
            data,
            file,
            probes: Arc::default(),
        },
        about,
    )
}

impl Rig {
    pub(crate) fn data_dir(&self) -> DataDir {
        DataDir::at(self.data.path())
    }

    /// Opens the Restore page of the open game from the File menu.
    pub(crate) fn open_page(&self) {
        self.ui.window.invoke_restore_menu_requested();
        assert!(self.ui.window.get_restore_open());
    }

    /// Clicks the row of the backup at `T0`, then Restore in the confirmation.
    /// Nothing has run: the next step is the worker, or the discard dialog.
    pub(crate) fn start_restore(&self) {
        self.ui
            .element("Restore the backup of 2020-09-13 12:26 UTC", Button)
            .invoke_accessible_default_action();
        assert!(self.ui.restore_bridge().get_confirm_open());
        self.ui.element("Restore", Button).invoke_accessible_default_action();
    }

    /// Backs up the rig's file as it is now, at `T0`, UTC.
    pub(crate) fn backup_now(&self, reason: &str) -> Backup {
        let folder = self.file.parent().unwrap();
        make_backup(&self.data_dir(), "ff1-pr", folder, &[slot_file(1)], reason, T0, 0)
    }

    /// The `created` text of the newest backup.
    pub(crate) fn newest_created(&self) -> String {
        let mut listing = Registry::new(Language::En).backups(&self.data_dir(), Game::Ff1Pr).unwrap();
        listing.backups.remove(0).manifest.created
    }

    pub(crate) fn backups(&self) -> usize {
        Registry::new(Language::En)
            .backups(&self.data_dir(), Game::Ff1Pr)
            .map_or(0, |listing| listing.backups.len())
    }

    pub(crate) fn bytes(&self) -> Vec<u8> {
        fs::read(&self.file).unwrap()
    }

    /// Another program saves gil `value` into the file: a second session.
    pub(crate) fn change_on_disk(&self, value: i64) {
        let mut other = self.session_on_disk();
        other.edit(&Edit::set(FieldId::new("gil"), Value::Int(value))).unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        other.save(&DataDir::at(elsewhere.path()), Running::Allow).unwrap();
    }

    /// The gil the file holds.
    pub(crate) fn gil_on_disk(&self) -> i64 {
        let view = self.session_on_disk().view().unwrap();
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

    fn session_on_disk(&self) -> Session {
        Registry::new(Language::En)
            .open(self.file.parent().unwrap(), self.file.file_name().unwrap().to_str().unwrap(), Some(Game::Ff1Pr))
            .unwrap()
    }
}
