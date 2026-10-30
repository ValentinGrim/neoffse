//! `EditorViewModel::save` when the save must not go through: a changed
//! file, a bad data folder, a second save, a panic, nothing open. Every save
//! writes into temporary folders.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::{Arc, Mutex, PoisonError};

use common::{ID_1, Part, editing, editing_site, field, saving, slot_bytes, slot_file, steam_folder};
use neoffse::Running;
use neoffse_app::editor::{Input, SaveOutcome};

type Log = Arc<Mutex<Vec<SaveOutcome>>>;

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
    saving.break_data_dir();
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
