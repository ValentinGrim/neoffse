//! `OpenViewModel::reopen`: the open save is read again from its file.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{ID_1, Part, editing, editing_site, field, registry, slot_file, steam_folder};
use neoffse::Running;
use neoffse::core::{Edit, FieldId, Game, Value};
use neoffse::io::dirs::DataDir;
use neoffse_app::OpenProblem;
use neoffse_app::editor::Input;
use neoffse_app::home::OpenOutcome;
use std::sync::{Arc, Mutex, PoisonError};

type Log = Arc<Mutex<Vec<OpenOutcome>>>;

fn recorder(log: &Log) -> impl FnOnce(OpenOutcome) + Send + 'static {
    let log = Arc::clone(log);
    move |outcome| log.lock().unwrap_or_else(PoisonError::into_inner).push(outcome)
}

fn outcomes(log: &Log) -> Vec<OpenOutcome> {
    log.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

#[test]
fn reopen_replaces_the_session_with_the_file_on_disk() {
    let site = editing_site();
    let ed = editing(&site);
    let key = field(&ed.data(), Part::Inventory, "Gil", "Gil").key;
    assert!(ed.editor().commit(key, Input::Text("500".to_owned())).is_none());
    let summary = ed.editor().summary().expect("a summary");

    // Another program changes the file: a second session saves gil 777.
    let folder = steam_folder(&site, 0, ID_1);
    let mut other = registry().open(&folder, slot_file(1), Some(Game::Ff1Pr)).unwrap();
    other.edit(&Edit::set(FieldId::new("gil"), Value::Int(777))).unwrap();
    let data = tempfile::tempdir().unwrap();
    other.save(&DataDir::at(data.path()), Running::Allow).unwrap();

    let log = Log::default();
    ed.flow.open.reopen(&ed.flow.manual.dispatcher(), recorder(&log));
    assert!(ed.flow.open.is_busy());
    ed.flow.manual.run_all();

    assert_eq!(outcomes(&log), [OpenOutcome::Opened]);
    assert!(!ed.flow.open.is_busy());
    assert_eq!(ed.editor().history().pending, 0);
    assert_eq!(field(&ed.data(), Part::Inventory, "Gil", "Gil").text.to_string(), "777");
    assert_eq!(ed.editor().summary().expect("a summary"), summary);
}

#[test]
fn reopen_keeps_the_old_session_when_the_file_is_gone() {
    let site = editing_site();
    let ed = editing(&site);
    let key = field(&ed.data(), Part::Inventory, "Gil", "Gil").key;
    assert!(ed.editor().commit(key, Input::Text("500".to_owned())).is_none());
    fs::remove_file(steam_folder(&site, 0, ID_1).join(slot_file(1))).unwrap();

    let log = Log::default();
    ed.flow.open.reopen(&ed.flow.manual.dispatcher(), recorder(&log));
    ed.flow.manual.run_all();

    assert_eq!(outcomes(&log), [OpenOutcome::Failed(OpenProblem::Unreadable)]);
    assert!(ed.editor().is_dirty());
    assert_eq!(ed.editor().history().pending, 1);
}

#[test]
fn reopen_without_a_save_open_fails_at_once() {
    let site = editing_site();
    let ed = editing(&site);
    ed.editor().back();

    let log = Log::default();
    ed.flow.open.reopen(&ed.flow.manual.dispatcher(), recorder(&log));
    assert_eq!(outcomes(&log), [OpenOutcome::Failed(OpenProblem::UnknownSlot)]);
    assert_eq!(ed.flow.manual.queued_jobs(), 0);
    assert!(!ed.flow.open.is_busy());
}

#[test]
fn reopen_while_busy_fails_with_busy() {
    let site = editing_site();
    let ed = editing(&site);
    let log = Log::default();
    ed.flow.open.reopen(&ed.flow.manual.dispatcher(), recorder(&log));
    assert!(ed.flow.open.is_busy());

    ed.flow.open.reopen(&ed.flow.manual.dispatcher(), recorder(&log));
    assert_eq!(outcomes(&log), [OpenOutcome::Failed(OpenProblem::Busy)]);
    assert_eq!(ed.flow.manual.queued_jobs(), 1, "only the first open queued a job");

    ed.flow.manual.run_all();
    assert_eq!(outcomes(&log), [OpenOutcome::Failed(OpenProblem::Busy), OpenOutcome::Opened]);
}
