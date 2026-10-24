//! A window over the first synthetic slot, wired for saves: a temporary data
//! folder and a process probe that counts its calls. Nothing here names the
//! platform data folder or the real process list.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use neoffse::core::{Capability, Edit, FieldId, Game, Value};
use neoffse::data::Language;
use neoffse::io::dirs::DataDir;
use neoffse::pr::games::GAMES;
use neoffse::{Registry, Running, Session};
use tempfile::TempDir;

use super::{ID_1, Site, Ui, open_editor, slot_file, steam_folder};

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
    let counter = Arc::clone(&probes);
    ui.app.editor.set_process_probe(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
        if machine == Machine::GameRuns {
            vec![GAMES[0].executable]
        } else {
            Vec::new()
        }
    });
    let file = steam_folder(site, 0, ID_1).join(slot_file(1));
    Rig { ui, data, file, probes }
}

impl Rig {
    pub(crate) fn data_dir(&self) -> DataDir {
        DataDir::at(self.data.path())
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
