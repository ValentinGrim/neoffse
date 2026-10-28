//! `EditorViewModel::save`: the pipeline on a worker, the stamp, the probe.
//! Every save writes into temporary folders.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use common::{editing_site, newest_created, saving, scripted_probe};
use neoffse::Running;
use neoffse_app::editor::{SaveOutcome, stamp_of};

#[test]
fn a_save_backs_up_writes_and_reports_a_stamp() {
    let site = editing_site();
    let saving = saving(&site);
    let before = fs::read(&saving.file).unwrap();
    saving.edit_gil("500");

    saving.save(Running::Refuse);
    assert!(saving.ed.editor().is_saving());
    saving.ed.flow.manual.run_all();

    let stamp = stamp_of(&newest_created(saving.data.path()));
    assert!(stamp.is_some());
    assert_eq!(saving.outcomes(), [SaveOutcome::Saved { stamp }]);
    assert_ne!(fs::read(&saving.file).unwrap(), before);
    assert_eq!(saving.gil_on_disk(), 500);
    let history = saving.ed.editor().history();
    assert_eq!(history.pending, 0);
    assert!(history.can_undo, "the history stays after a save");
    assert!(!saving.ed.editor().is_saving());
}

#[test]
fn the_work_runs_off_the_calling_thread() {
    let site = editing_site();
    let saving = saving(&site);
    saving.edit_gil("500");
    saving.save(Running::Refuse);

    let jobs = saving.ed.flow.manual.run_jobs();
    assert_eq!(jobs.len(), 1);
    assert_ne!(jobs[0], thread::current().id());
    assert_eq!(saving.outcomes().len(), 0, "done waits for the UI thread");
    saving.ed.flow.manual.run_posts();
    assert_eq!(saving.threads(), [thread::current().id()]);
}

#[test]
fn a_clean_session_saves_nothing() {
    let site = editing_site();
    let saving = saving(&site);
    let before = fs::read(&saving.file).unwrap();
    saving.save(Running::Refuse);
    saving.ed.flow.manual.run_all();
    assert_eq!(saving.outcomes(), [SaveOutcome::Saved { stamp: None }]);
    assert!(!saving.data.path().join("backups").exists());
    assert_eq!(fs::read(&saving.file).unwrap(), before);
}

#[test]
fn an_undo_after_a_save_makes_the_session_dirty_again() {
    let site = editing_site();
    let saving = saving(&site);
    saving.edit_gil("500");
    saving.save(Running::Refuse);
    saving.ed.flow.manual.run_all();

    assert!(saving.ed.editor().undo().is_none());
    let history = saving.ed.editor().history();
    assert_eq!(history.pending, 1);
    assert!(history.can_redo, "the undone step can come back");
    assert!(!history.can_undo);

    saving.save(Running::Refuse);
    saving.ed.flow.manual.run_all();
    assert!(matches!(saving.outcomes().last(), Some(SaveOutcome::Saved { stamp: Some(_) })));
    assert_eq!(saving.gil_on_disk(), 123_456);
}

#[test]
fn a_running_game_is_refused_and_nothing_is_written() {
    let site = editing_site();
    let saving = saving(&site);
    let calls = Arc::new(AtomicUsize::new(0));
    let runs = Arc::new(AtomicBool::new(true));
    saving.ed.editor().set_process_probe(scripted_probe(Arc::clone(&calls), runs));
    let before = fs::read(&saving.file).unwrap();
    saving.edit_gil("500");

    saving.save(Running::Refuse);
    saving.ed.flow.manual.run_all();
    assert_eq!(saving.outcomes(), [SaveOutcome::GameRunning]);
    assert_eq!(fs::read(&saving.file).unwrap(), before);
    assert_eq!(saving.backups(), 0);
    assert!(saving.ed.editor().is_dirty());

    saving.save(Running::Allow);
    saving.ed.flow.manual.run_all();
    assert!(matches!(saving.outcomes().last(), Some(SaveOutcome::Saved { stamp: Some(_) })));
    assert_eq!(calls.load(Ordering::SeqCst), 1, "Allow never probes");
    assert_eq!(saving.gil_on_disk(), 500);
}

#[test]
fn allow_never_probes() {
    let site = editing_site();
    let saving = saving(&site);
    let calls = Arc::new(AtomicUsize::new(0));
    let runs = Arc::new(AtomicBool::new(false));
    saving.ed.editor().set_process_probe(scripted_probe(Arc::clone(&calls), runs));
    saving.edit_gil("500");
    saving.save(Running::Allow);
    saving.ed.flow.manual.run_all();
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(matches!(saving.outcomes()[0], SaveOutcome::Saved { .. }));
}
