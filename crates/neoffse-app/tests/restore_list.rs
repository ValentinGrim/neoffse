//! The Restore view model's list: order, counts, failures, selection.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::Arc;

use common::{Restoring, T0, make_backup, registry};
use neoffse::core::Game;
use neoffse::io::dirs::DataDir;
use neoffse_app::state::AppState;
use neoffse_app::{RestoreProblem, RestoreViewModel};

#[test]
fn show_lists_newest_first_and_counts_unreadable_folders() {
    let r = Restoring::new();
    let backups: Vec<_> = (0..3).map(|minutes| r.backup_now("before-save", minutes)).collect();
    // A stamp-named folder with no backup.json.
    fs::create_dir(backups[0].dir.parent().unwrap().join("20200101-000000")).unwrap();

    assert!(r.vm.show(Game::Ff1Pr).is_none());
    let stamps: Vec<String> = r.vm.rows().iter().map(|row| row.stamp.to_string()).collect();
    assert_eq!(stamps, ["2020-09-13 12:28 UTC", "2020-09-13 12:27 UTC", "2020-09-13 12:26 UTC"]);
    let keys: Vec<i32> = r.vm.rows().iter().map(|row| row.key).collect();
    assert_eq!(keys, [0, 1, 2]);
    assert_eq!(r.vm.unreadable_count(), 1);
    assert_eq!(r.vm.title(), "Final Fantasy I");
    assert_eq!(r.vm.game(), Some(Game::Ff1Pr));
}

#[test]
fn a_game_without_backups_lists_nothing() {
    let r = Restoring::new();
    assert!(r.vm.show(Game::Ff1Pr).is_none());
    assert_eq!(r.vm.rows().len(), 0);
    assert_eq!(r.vm.unreadable_count(), 0);
    assert_eq!(fs::read_dir(r.data.path()).unwrap().count(), 0, "listing creates nothing");
}

#[test]
fn no_data_folder_is_a_listing_failure() {
    let vm = RestoreViewModel::new(registry(), Arc::new(AppState::new()));
    let notice = vm.show(Game::Ff1Pr).unwrap();
    assert_eq!(notice.problem, RestoreProblem::ListingFailed);
    assert_eq!(notice.value.as_str(), "");
    assert_eq!(vm.rows().len(), 0);
    assert_eq!(vm.game(), Some(Game::Ff1Pr));
}

#[test]
fn a_data_folder_that_is_a_file_is_a_listing_failure() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("data");
    fs::write(&file, b"not a folder").unwrap();
    let state = Arc::new(AppState::new());
    state.set_data_dir(DataDir::at(&file));
    let vm = RestoreViewModel::new(registry(), state);
    assert_eq!(vm.show(Game::Ff1Pr).unwrap().problem, RestoreProblem::ListingFailed);
    assert_eq!(vm.rows().len(), 0);
}

#[test]
fn a_listing_failure_empties_a_list_that_was_shown() {
    let r = Restoring::new();
    r.backup_now("before-save", 0);
    assert!(r.vm.show(Game::Ff1Pr).is_none());
    assert_eq!(r.vm.rows().len(), 1);
    let file = tempfile::tempdir().unwrap();
    let path = file.path().join("data");
    fs::write(&path, b"x").unwrap();
    r.flow.state.set_data_dir(DataDir::at(&path));
    assert!(r.vm.show(Game::Ff1Pr).is_some());
    assert_eq!(r.vm.rows().len(), 0);
}

#[test]
fn show_clears_the_selection() {
    let r = Restoring::new();
    r.backup_now("before-save", 0);
    r.pick(0);
    assert!(r.vm.selected().is_some());
    assert!(r.vm.show(Game::Ff1Pr).is_none());
    assert!(r.vm.selected().is_none());
}

#[test]
fn select_returns_the_row_and_an_unknown_key_clears_it() {
    let r = Restoring::new();
    r.backup_now("before-save", 0);
    r.backup_now("before-restore", 1);
    assert!(r.vm.show(Game::Ff1Pr).is_none());

    let row = r.vm.select(1).unwrap();
    assert_eq!(row.key, 1);
    assert_eq!(row.stamp.as_str(), "2020-09-13 12:26 UTC");
    assert_eq!(r.vm.selected().unwrap().key, 1);

    assert!(r.vm.select(2).is_none());
    assert!(r.vm.selected().is_none());
    assert!(r.vm.select(0).is_some());
    assert!(r.vm.select(-1).is_none());
    assert!(r.vm.selected().is_none());
}

#[test]
fn another_game_has_its_own_list() {
    let r = Restoring::new();
    r.backup_now("before-save", 0);
    let other = tempfile::tempdir().unwrap();
    fs::write(other.path().join("notes.bin"), b"arbitrary").unwrap();
    make_backup(&r.data_dir(), "ff2-pr", other.path(), &["notes.bin"], "before-save", T0, 0);
    make_backup(&r.data_dir(), "ff2-pr", other.path(), &["notes.bin"], "before-save", T0 + 60, 0);

    assert!(r.vm.show(Game::Ff1Pr).is_none());
    assert_eq!(r.vm.rows().len(), 1);
    assert!(r.vm.show(Game::Ff2Pr).is_none());
    assert_eq!(r.vm.rows().len(), 2);
    assert_eq!(r.vm.title(), "Final Fantasy II");
    assert_eq!(r.vm.rows()[0].file.as_str(), "notes.bin");
}

#[test]
fn close_forgets_the_list_and_the_selection() {
    let r = Restoring::new();
    r.backup_now("before-save", 0);
    r.pick(0);
    r.vm.close();
    assert_eq!(r.vm.rows().len(), 0);
    assert!(r.vm.selected().is_none());
    assert!(r.vm.game().is_none());
    assert_eq!(r.vm.title(), "");
}

#[test]
fn debug_holds_no_path() {
    let r = Restoring::new();
    r.backup_now("before-save", 0);
    r.pick(0);
    let text = format!("{:?}", r.vm);
    assert!(!text.contains(r.folder.to_str().unwrap()), "Debug holds the save folder");
    assert!(!text.contains(r.data.path().to_str().unwrap()), "Debug holds the data folder");
    assert!(text.contains("ff1-pr"));
}
