//! The open flow from the file dialog: any file, no game given.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use common::{Flow, ID_1, Outcomes, Refusing, Site, no_game_bytes, other_game_bytes, registry_with, slot_bytes, slot_file, steam_slot};
use neoffse::Registry;
use neoffse::core::Game;
use neoffse_app::OpenProblem;
use neoffse_app::home::OpenOutcome;

const FF1: usize = 0;

fn failed(problem: OpenProblem) -> OpenOutcome {
    OpenOutcome::Failed(problem)
}

/// Writes `bytes` as `name` in a folder no scan covers.
fn elsewhere(site: &Site, name: &str, bytes: &[u8]) -> PathBuf {
    let dir = site.root().join("elsewhere");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, bytes).unwrap();
    path
}

/// Asks the picker once and returns how it ended.
fn pick_once(flow: &Flow) -> Vec<OpenOutcome> {
    let outcomes = Outcomes::new();
    flow.open.open_file(&flow.manual.dispatcher(), outcomes.recorder());
    flow.manual.run_all();
    outcomes.outcomes()
}

#[test]
fn open_file_opens_the_chosen_slot() {
    let site = Site::new();
    let path = elsewhere(&site, slot_file(1), &slot_bytes(1));
    let flow = Flow::new(&site, Some(path));

    assert_eq!(pick_once(&flow), vec![OpenOutcome::Opened]);
    assert_eq!(flow.state.summary().unwrap().party.as_str(), "Aria, Bram, Cleo, Zoé");
    let calls = flow.picker.calls();
    assert_eq!(calls.len(), 1);
    assert_ne!(calls[0], thread::current().id());
}

#[test]
fn a_renamed_slot_opens_with_no_game_given() {
    let site = Site::new();
    let path = elsewhere(&site, "renamed.bin", &slot_bytes(1));
    let flow = Flow::new(&site, Some(path));

    assert_eq!(pick_once(&flow), vec![OpenOutcome::Opened]);
    assert!(flow.state.is_open());
}

#[test]
fn cancel_changes_nothing() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let flow = Flow::new(&site, None);
    assert_eq!(pick_once(&flow), vec![OpenOutcome::Cancelled]);
    assert!(!flow.state.is_open());

    flow.click(slot_file(1), &Outcomes::new());
    flow.manual.run_all();
    let kept = flow.state.summary().unwrap();

    assert_eq!(pick_once(&flow), vec![OpenOutcome::Cancelled]);
    assert_eq!(flow.state.summary().unwrap(), kept);
    assert!(!flow.open.is_busy());
    assert_eq!(pick_once(&flow), vec![OpenOutcome::Cancelled], "a second dialog opens");
    assert_eq!(flow.picker.calls().len(), 3);
}

#[test]
fn a_garbage_file_gives_not_a_save() {
    let site = Site::new();
    let path = elsewhere(&site, "garbage.bin", b"not a save");
    let flow = Flow::new(&site, Some(path));
    assert_eq!(pick_once(&flow), vec![failed(OpenProblem::NotASave)]);
    assert!(!flow.state.is_open());
}

#[test]
fn a_missing_file_gives_unreadable() {
    let site = Site::new();
    let flow = Flow::new(&site, Some(site.root().join("elsewhere").join("absent.bin")));
    assert_eq!(pick_once(&flow), vec![failed(OpenProblem::Unreadable)]);
    assert!(!flow.state.is_open());
}

#[test]
fn a_path_with_no_file_name_gives_unreadable() {
    let site = Site::new();
    let folder = Flow::new(&site, Some(site.root()));
    assert_eq!(pick_once(&folder), vec![failed(OpenProblem::Unreadable)]);
    assert!(!folder.open.is_busy());

    #[cfg(unix)]
    {
        let root = Flow::new(&site, Some(PathBuf::from("/")));
        assert_eq!(pick_once(&root), vec![failed(OpenProblem::Unreadable)]);
    }
}

#[test]
fn the_dialog_is_not_asked_while_busy() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let flow = Flow::new(&site, None);
    flow.click(slot_file(1), &Outcomes::new());

    let outcomes = Outcomes::new();
    flow.open.open_file(&flow.manual.dispatcher(), outcomes.recorder());
    assert_eq!(outcomes.all(), vec![(failed(OpenProblem::Busy), thread::current().id())]);
    assert_eq!(flow.manual.queued_jobs(), 1, "only the slot open is queued");

    flow.manual.run_all();
    assert_eq!(flow.picker.calls().len(), 0);
}

#[test]
fn a_slot_of_another_game_gives_unsupported() {
    let site = Site::new();
    let path = elsewhere(&site, slot_file(1), &other_game_bytes());
    let flow = Flow::new(&site, Some(path));

    assert_eq!(pick_once(&flow), vec![failed(OpenProblem::Unsupported)]);
    assert!(!flow.state.is_open());
}

#[test]
fn a_vehicle_count_no_game_has_stays_not_a_save() {
    let site = Site::new();
    let path = elsewhere(&site, slot_file(1), &no_game_bytes());
    let flow = Flow::new(&site, Some(path));

    assert_eq!(pick_once(&flow), vec![failed(OpenProblem::NotASave)]);
    assert!(!flow.state.is_open());
}

#[test]
fn a_file_that_names_a_registered_game_gives_not_a_save() {
    let site = Site::new();
    let path = elsewhere(&site, slot_file(1), &other_game_bytes());
    let refusing = Refusing {
        game: Game::Ff6Pr,
        found: Some(Game::Ff1Pr),
        vehicles: 4,
    };
    let flow = Flow::with_registry(&site, Some(path), registry_with(vec![Arc::new(refusing)]));

    assert_eq!(pick_once(&flow), vec![failed(OpenProblem::NotASave)]);
    assert!(!flow.state.is_open());
}

#[test]
fn a_slot_of_an_unregistered_game_stays_unsupported_with_two_formats() {
    let site = Site::new();
    let path = elsewhere(&site, slot_file(1), &other_game_bytes());
    let refusing = Refusing {
        game: Game::Ff2Pr,
        found: Some(Game::Ff6Pr),
        vehicles: 4,
    };
    let flow = Flow::with_registry(&site, Some(path), registry_with(vec![Arc::new(refusing)]));

    assert_eq!(pick_once(&flow), vec![failed(OpenProblem::Unsupported)]);
    assert!(!flow.state.is_open());
}

#[test]
fn an_empty_registry_gives_not_a_save() {
    let site = Site::new();
    let path = elsewhere(&site, slot_file(1), &slot_bytes(1));
    let flow = Flow::with_registry(&site, Some(path), Arc::new(Registry::from_formats(Vec::new())));

    assert_eq!(pick_once(&flow), vec![failed(OpenProblem::NotASave)]);
    assert!(!flow.state.is_open());
}
