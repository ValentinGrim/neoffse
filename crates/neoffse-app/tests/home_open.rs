//! The open flow from a click on a slot: the worker thread, the shared
//! session, the failures and Back.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::Arc;
use std::thread;

use common::{Flow, ID_1, Outcomes, Site, slot_bytes, slot_file, slots_of, steam_slot};
use neoffse::core::Game;
use neoffse_app::OpenProblem;
use neoffse_app::home::{OpenOutcome, OpenViewModel};
use neoffse_app::state::{AppState, EditorViewModel};

const FF1: usize = 0;
const FF2: usize = 1;

fn failed(problem: OpenProblem) -> OpenOutcome {
    OpenOutcome::Failed(problem)
}

/// A site with slot 1 (valid) of FF I.
fn one_slot() -> (Site, std::path::PathBuf) {
    let site = Site::new();
    let dir = steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    (site, dir)
}

#[test]
fn a_click_opens_the_session_off_the_ui_thread() {
    let (site, _) = one_slot();
    let flow = Flow::new(&site, None);
    let outcomes = Outcomes::new();
    flow.click(slot_file(1), &outcomes);

    assert!(!flow.state.is_open());
    assert!(outcomes.all().is_empty(), "done ran before the job");
    assert!(flow.open.is_busy());
    assert_eq!(flow.manual.queued_jobs(), 1);

    let workers = flow.manual.run_jobs();
    assert!(outcomes.all().is_empty(), "done ran on the worker");
    flow.manual.run_posts();

    let all = outcomes.all();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].0, OpenOutcome::Opened);
    assert_eq!(all[0].1, thread::current().id());
    assert_eq!(workers.len(), 1);
    assert_ne!(workers[0], thread::current().id());
    assert!(flow.state.is_open());
    assert!(!flow.open.is_busy());
}

#[test]
fn the_state_holds_a_shared_session() {
    let (site, _) = one_slot();
    let flow = Flow::new(&site, None);
    let outcomes = Outcomes::new();
    flow.click(slot_file(1), &outcomes);
    flow.manual.run_all();

    let session = flow.state.session().expect("a session is open");
    {
        let guard = session.lock().unwrap();
        assert_eq!(guard.game(), Game::Ff1Pr);
        assert!(!guard.is_dirty());
    }
    let home_rows = flow.home.rows();
    let slot = slots_of(&home_rows[0]).remove(0);
    assert_eq!(flow.state.summary().unwrap(), slot.summary);
    assert_eq!(flow.editor.summary().unwrap(), slot.summary);
    assert_eq!(slot.summary.party.as_str(), "Aria, Bram, Cleo, Zoé");
}

#[test]
fn the_file_is_untouched() {
    let (site, dir) = one_slot();
    let path = dir.join(slot_file(1));
    let names = |dir: &std::path::Path| {
        let mut names: Vec<_> = fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name()).collect();
        names.sort();
        names
    };
    let before = (fs::read(&path).unwrap(), names(&dir));

    let flow = Flow::new(&site, None);
    flow.click(slot_file(1), &Outcomes::new());
    flow.manual.run_all();

    assert!(flow.state.is_open());
    assert_eq!((fs::read(&path).unwrap(), names(&dir)), before);
}

#[test]
fn an_unknown_key_fails_without_a_job() {
    let (site, _) = one_slot();
    let flow = Flow::new(&site, None);
    let outcomes = Outcomes::new();
    flow.open.open_slot(None, &flow.manual.dispatcher(), outcomes.recorder());

    let all = outcomes.all();
    assert_eq!(all.len(), 1, "done runs at once");
    assert_eq!(all[0], (failed(OpenProblem::UnknownSlot), thread::current().id()));
    assert_eq!(flow.manual.queued_jobs(), 0);
    assert!(!flow.open.is_busy());
}

#[test]
fn a_game_without_a_format_is_refused_without_a_job() {
    let site = Site::new();
    steam_slot(&site, FF2, ID_1, slot_file(1), &slot_bytes(1));
    let flow = Flow::new(&site, None);
    let outcomes = Outcomes::new();
    flow.click(slot_file(1), &outcomes);

    assert_eq!(outcomes.outcomes(), vec![failed(OpenProblem::Unsupported)]);
    assert_eq!(flow.manual.queued_jobs(), 0);
    assert!(!flow.open.is_busy());
    assert!(!flow.state.is_open());
}

#[test]
fn a_garbage_slot_gives_not_a_save() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    steam_slot(&site, FF1, ID_1, slot_file(2), b"not a save");
    let flow = Flow::new(&site, None);

    let outcomes = Outcomes::new();
    flow.click(slot_file(2), &outcomes);
    flow.manual.run_all();
    assert_eq!(outcomes.outcomes(), vec![failed(OpenProblem::NotASave)]);
    assert!(!flow.state.is_open());
    assert!(!flow.open.is_busy());

    flow.click(slot_file(1), &outcomes);
    flow.manual.run_all();
    let kept = flow.state.summary().expect("slot 1 is open");

    flow.click(slot_file(2), &outcomes);
    flow.manual.run_all();
    assert_eq!(outcomes.outcomes().last(), Some(&failed(OpenProblem::NotASave)));
    assert!(flow.state.is_open(), "a failed open keeps the open save");
    assert_eq!(flow.state.summary().unwrap(), kept);
}

#[test]
fn a_file_that_vanished_gives_unreadable() {
    let (site, dir) = one_slot();
    let flow = Flow::new(&site, None);
    fs::remove_file(dir.join(slot_file(1))).unwrap();

    let outcomes = Outcomes::new();
    flow.click(slot_file(1), &outcomes);
    flow.manual.run_all();
    assert_eq!(outcomes.outcomes(), vec![failed(OpenProblem::Unreadable)]);
    assert!(!flow.state.is_open());
}

#[test]
fn a_second_open_during_an_open_is_busy() {
    let (site, _) = one_slot();
    let flow = Flow::new(&site, None);
    let first = Outcomes::new();
    let second = Outcomes::new();
    flow.click(slot_file(1), &first);
    flow.click(slot_file(1), &second);

    assert_eq!(second.all(), vec![(failed(OpenProblem::Busy), thread::current().id())]);
    assert_eq!(flow.manual.queued_jobs(), 1);

    flow.manual.run_all();
    assert_eq!(first.outcomes(), vec![OpenOutcome::Opened]);
    assert!(!flow.open.is_busy());

    let third = Outcomes::new();
    flow.click(slot_file(1), &third);
    flow.manual.run_all();
    assert_eq!(third.outcomes(), vec![OpenOutcome::Opened]);
}

#[test]
fn back_drops_the_session() {
    let (site, _) = one_slot();
    let flow = Flow::new(&site, None);
    flow.click(slot_file(1), &Outcomes::new());
    flow.manual.run_all();

    let weak = Arc::downgrade(&flow.state.session().unwrap());
    assert!(weak.upgrade().is_some());
    flow.editor.back();

    assert!(!flow.state.is_open());
    assert!(flow.editor.summary().is_none());
    assert!(weak.upgrade().is_none(), "the session is still alive");
}

#[test]
fn a_new_open_replaces_the_old_one() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    steam_slot(&site, FF1, ID_1, slot_file(2), &slot_bytes(2));
    let flow = Flow::new(&site, None);

    flow.click(slot_file(1), &Outcomes::new());
    flow.manual.run_all();
    let first = Arc::downgrade(&flow.state.session().unwrap());
    flow.click(slot_file(2), &Outcomes::new());
    flow.manual.run_all();

    let home_rows = flow.home.rows();
    let slots = slots_of(&home_rows[0]);
    assert_eq!(slots.len(), 2);
    assert_eq!(flow.state.summary().unwrap(), slots[1].summary);
    assert_eq!(flow.state.summary().unwrap().number, 2);
    assert!(first.upgrade().is_none(), "the replaced session is still alive");
}

#[test]
fn state_types_are_send_and_sync() {
    fn needs<T: Send + Sync>() {}
    needs::<AppState>();
    needs::<OpenViewModel>();
    needs::<EditorViewModel>();
}

#[test]
fn debug_prints_no_name_and_no_path() {
    let (site, dir) = one_slot();
    let flow = Flow::new(&site, None);
    flow.click(slot_file(1), &Outcomes::new());
    flow.manual.run_all();

    let printed = format!("{:?} {:?} {:?}", flow.state, flow.open, flow.editor);
    assert!(printed.contains("ff1-pr"));
    assert!(!printed.contains(slot_file(1)));
    assert!(!printed.contains(dir.to_str().unwrap()));
    assert!(!printed.contains("Aria"));
}
