//! `EditorViewModel::save` when the save must not go through: a changed
//! file, a bad data folder, a second save, a panic, nothing open. Every save
//! writes into temporary folders.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use common::{Editing, ID_1, Part, Site, editing, editing_site, field, slot_bytes, slot_file, steam_folder};
use neoffse::Running;
use neoffse::io::dirs::DataDir;
use neoffse_app::editor::{Input, SaveOutcome};
use tempfile::TempDir;

type Log = Arc<Mutex<Vec<SaveOutcome>>>;

struct Saving {
    ed: Editing,
    data: TempDir,
    file: PathBuf,
    log: Log,
}

/// An open slot, a temporary data folder and a probe that finds no game.
fn saving(site: &Site) -> Saving {
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
    fn edit_gil(&self, value: &str) {
        let key = field(&self.ed.data(), Part::Inventory, "Gil", "Gil").key;
        assert!(self.ed.editor().commit(key, Input::Text(value.to_owned())).is_none());
    }

    fn save(&self, running: Running) {
        let log = Arc::clone(&self.log);
        self.ed.editor().save(running, &self.ed.flow.manual.dispatcher(), move |outcome| {
            log.lock().unwrap_or_else(PoisonError::into_inner).push(outcome);
        });
    }

    fn outcomes(&self) -> Vec<SaveOutcome> {
        self.log.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

#[test]
fn a_file_changed_on_disk_is_refused() {
    let site = editing_site();
    let saving = saving(&site);
    saving.edit_gil("500");
    let foreign = slot_bytes(2);
    fs::write(&saving.file, &foreign).unwrap();

    saving.save(Running::Refuse);
    saving.ed.flow.manual.run_all();
    assert_eq!(saving.outcomes(), [SaveOutcome::ChangedOnDisk]);
    assert_eq!(fs::read(&saving.file).unwrap(), foreign);
    assert!(!saving.data.path().join("backups").exists());
    assert!(saving.ed.editor().is_dirty());
    assert!(!saving.ed.editor().is_saving());
}

#[test]
fn a_failed_write_is_a_failed_outcome() {
    let site = editing_site();
    let saving = saving(&site);
    let not_a_folder = saving.data.path().join("plain-file");
    fs::write(&not_a_folder, b"x").unwrap();
    saving.ed.flow.state.set_data_dir(DataDir::at(&not_a_folder));
    let before = fs::read(&saving.file).unwrap();
    saving.edit_gil("500");

    saving.save(Running::Refuse);
    saving.ed.flow.manual.run_all();
    assert_eq!(saving.outcomes(), [SaveOutcome::Failed]);
    assert!(saving.ed.editor().is_dirty());
    assert_eq!(fs::read(&saving.file).unwrap(), before);
    assert!(!saving.ed.editor().is_saving());
}

#[test]
fn no_data_dir_fails_at_once() {
    let site = editing_site();
    let ed = editing(&site);
    let file = steam_folder(&site, 0, ID_1).join(slot_file(1));
    let before = fs::read(&file).unwrap();
    let key = field(&ed.data(), Part::Inventory, "Gil", "Gil").key;
    assert!(ed.editor().commit(key, Input::Text("500".to_owned())).is_none());

    let log = Log::default();
    let sink = Arc::clone(&log);
    ed.editor().save(Running::Allow, &ed.flow.manual.dispatcher(), move |outcome| {
        sink.lock().unwrap_or_else(PoisonError::into_inner).push(outcome);
    });
    assert_eq!(*log.lock().unwrap_or_else(PoisonError::into_inner), [SaveOutcome::Failed], "before any run_all");
    assert_eq!(ed.flow.manual.queued_jobs(), 0);
    assert!(!ed.editor().is_saving());
    assert_eq!(fs::read(&file).unwrap(), before);
}

#[test]
fn a_second_save_while_one_runs_is_busy() {
    let site = editing_site();
    let saving = saving(&site);
    saving.edit_gil("500");
    saving.save(Running::Refuse);
    assert_eq!(saving.outcomes().len(), 0);

    saving.save(Running::Refuse);
    assert_eq!(saving.outcomes(), [SaveOutcome::Busy]);
    assert!(saving.ed.editor().is_saving(), "the first save still holds the flag");

    saving.ed.flow.manual.run_all();
    let outcomes = saving.outcomes();
    assert_eq!(outcomes.len(), 2);
    assert!(matches!(outcomes[1], SaveOutcome::Saved { stamp: Some(_) }));
    assert!(!saving.ed.editor().is_saving());
}

#[test]
fn a_panicking_probe_is_a_failure() {
    let site = editing_site();
    let saving = saving(&site);
    saving.ed.editor().set_process_probe(|_| panic!("probe failure injected by the test"));
    saving.edit_gil("500");

    saving.save(Running::Refuse);
    saving.ed.flow.manual.run_all();
    assert_eq!(saving.outcomes(), [SaveOutcome::Failed]);
    assert!(!saving.ed.editor().is_saving());
    assert_eq!(saving.ed.editor().history().pending, 1, "the poisoned lock is recovered");
}

#[test]
fn no_session_fails_at_once() {
    let site = editing_site();
    let saving = saving(&site);
    saving.ed.editor().back();

    saving.save(Running::Allow);
    assert_eq!(saving.outcomes(), [SaveOutcome::Failed]);
    assert_eq!(saving.ed.flow.manual.queued_jobs(), 0);
    assert!(!saving.ed.editor().is_saving());
}
