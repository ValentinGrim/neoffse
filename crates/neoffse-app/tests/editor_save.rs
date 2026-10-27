//! `EditorViewModel::save`: the pipeline on a worker, the stamp, the probe.
//! Every save writes into temporary folders.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};

use common::{Editing, ID_1, Part, Site, editing, editing_site, field, scripted_probe, slot_file, steam_folder};
use neoffse::Running;
use neoffse::core::{Capability, FieldId, Game, Value};
use neoffse::data::Language;
use neoffse::io::dirs::DataDir;
use neoffse::{Registry, Session};
use neoffse_app::editor::{Input, SaveOutcome, stamp_of};
use tempfile::TempDir;

type Log = Arc<Mutex<Vec<(SaveOutcome, ThreadId)>>>;

/// An open slot with a temporary data folder and a probe that finds no game.
struct Saving {
    ed: Editing,
    data: TempDir,
    file: PathBuf,
    log: Log,
}

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
            log.lock().unwrap_or_else(PoisonError::into_inner).push((outcome, thread::current().id()));
        });
    }

    fn outcomes(&self) -> Vec<SaveOutcome> {
        self.log
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(|(outcome, _)| outcome.clone())
            .collect()
    }

    fn backups(&self) -> usize {
        let data = DataDir::at(self.data.path());
        Registry::new(Language::En)
            .backups(&data, Game::Ff1Pr)
            .map_or(0, |listing| listing.backups.len())
    }

    fn gil_on_disk(&self) -> i64 {
        let mut session: Session = Registry::new(Language::En)
            .open(self.file.parent().unwrap(), self.file.file_name().unwrap().to_str().unwrap(), Some(Game::Ff1Pr))
            .unwrap();
        let view = session.view().unwrap();
        let entity = view
            .section(Capability::Inventory)
            .unwrap()
            .entities
            .iter()
            .find(|entity| entity.id == FieldId::new("gil"))
            .unwrap();
        match entity.fields[0].value {
            Value::Int(gil) => gil,
            ref other => panic!("the gil is not an integer: {other:?}"),
        }
    }
}

#[test]
fn a_save_backs_up_writes_and_reports_a_stamp() {
    let site = editing_site();
    let saving = saving(&site);
    let before = fs::read(&saving.file).unwrap();
    saving.edit_gil("500");

    saving.save(Running::Refuse);
    assert!(saving.ed.editor().is_saving());
    saving.ed.flow.manual.run_all();

    let newest = {
        let data = DataDir::at(saving.data.path());
        Registry::new(Language::En).backups(&data, Game::Ff1Pr).unwrap().backups.remove(0)
    };
    let stamp = stamp_of(&newest.manifest.created);
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
    let log = saving.log.lock().unwrap_or_else(PoisonError::into_inner);
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].1, thread::current().id());
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
