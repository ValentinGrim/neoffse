//! The owned-only filter, the scan flag and the way a scan runs through the
//! dispatcher.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use common::{ID_1, ID_2, Manual, Site, documents_slot, environment, install, items, registry, scanned, slot_bytes, slot_file, slots_of, steam_slot};
use neoffse_app::home::HomeViewModel;
use neoffse_app::{GameRow, SlotRow};
use slint::Model;

const FF1: usize = 0;
const FF2: usize = 1;

fn view_model(site: &Site) -> HomeViewModel {
    HomeViewModel::new(registry(), environment(site))
}

fn keys_of(row: &GameRow) -> Vec<i32> {
    let mut slots: Vec<SlotRow> = slots_of(row);
    for folder in items(&row.folders) {
        slots.extend(items(&folder.slots));
    }
    slots.iter().map(|slot| slot.key).collect()
}

#[test]
fn owned_only_starts_true_and_is_not_persisted() {
    let site = Site::new();
    let first = view_model(&site);
    assert!(first.owned_only());
    first.set_owned_only(false);
    assert!(!first.owned_only());
    assert!(view_model(&site).owned_only());
}

#[test]
fn before_the_first_scan_rows_are_empty() {
    let site = Site::new();
    let home = view_model(&site);
    assert_eq!(home.rows().len(), 0);
    home.set_owned_only(false);
    assert_eq!(home.rows().len(), 0);
    assert!(!home.is_scanning());
}

#[test]
fn a_game_is_owned_by_a_save_folder_alone() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let rows = scanned(&site).rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title.as_str(), "Final Fantasy I");
}

#[test]
fn a_game_is_owned_by_an_install_alone() {
    let site = Site::new();
    install(&site, FF2);
    let rows = scanned(&site).rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].title.as_str(), "Final Fantasy II");
    assert!(!rows[0].supported);
    assert_eq!(rows[0].slots.row_count() + rows[0].folders.row_count(), 0);
    assert!(!rows[0].no_save);

    let site = Site::new();
    install(&site, FF1);
    let rows = scanned(&site).rows();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].supported && rows[0].no_save);
}

#[test]
fn an_unowned_game_hides_when_the_filter_is_on() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let home = scanned(&site);
    assert_eq!(home.rows().len(), 1);
    home.set_owned_only(false);
    assert_eq!(home.rows().len(), 6);
}

#[test]
fn hiding_a_game_hides_its_folders_and_slots() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    documents_slot(&site, FF1, ID_1, slot_file(2), &slot_bytes(2));
    let home = scanned(&site);

    let before = keys_of(&home.rows()[0]);
    assert_eq!(before.len(), 2);
    let rows = home.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].folders.row_count(), 2);

    home.set_owned_only(false);
    let rows = home.rows();
    assert_eq!(rows.len(), 6);
    assert_eq!(keys_of(&rows[0]), before);

    home.set_owned_only(true);
    let rows = home.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(keys_of(&rows[0]), before);
}

#[test]
fn a_rescan_replaces_the_data() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let home = scanned(&site);
    assert_eq!(slots_of(&home.rows()[0]).len(), 1);

    steam_slot(&site, FF1, ID_1, slot_file(2), &slot_bytes(2));
    let manual = Manual::new();
    home.start_scan(&manual.dispatcher(), || {});
    manual.run_all();
    assert_eq!(slots_of(&home.rows()[0]).len(), 2);
}

#[test]
fn the_scan_does_not_block_the_caller() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let home = view_model(&site);
    let manual = Manual::new();
    let ran = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&ran);
    home.start_scan(&manual.dispatcher(), move || log.lock().unwrap().push(thread::current().id()));

    assert!(home.is_scanning());
    assert_eq!(home.rows().len(), 0);
    assert!(ran.lock().unwrap().is_empty());

    manual.run_all();
    assert!(!home.is_scanning());
    assert_eq!(*ran.lock().unwrap(), [thread::current().id()]);
    assert_eq!(home.rows().len(), 1);
}

#[test]
fn the_work_runs_off_the_test_thread() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let home = view_model(&site);
    let manual = Manual::new();
    home.start_scan(&manual.dispatcher(), || {});

    let used = manual.run_jobs();
    assert_eq!(used.len(), 1);
    assert_ne!(used[0], thread::current().id());
    // The data is still not in place: the post has not run.
    assert_eq!(home.rows().len(), 0);
    manual.run_posts();
    assert_eq!(home.rows().len(), 1);
}

#[test]
fn a_second_start_during_a_scan_is_ignored() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    steam_slot(&site, FF1, ID_2, slot_file(1), &slot_bytes(1));
    let home = view_model(&site);
    let manual = Manual::new();
    let count = Arc::new(AtomicUsize::new(0));
    for _ in 0..2 {
        let count = Arc::clone(&count);
        home.start_scan(&manual.dispatcher(), move || {
            count.fetch_add(1, Ordering::SeqCst);
        });
    }
    assert_eq!(manual.queued_jobs(), 1);
    manual.run_all();
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert!(!home.is_scanning());
}

#[test]
fn a_finished_scan_is_not_failed() {
    let site = Site::new();
    let home = view_model(&site);
    assert!(!home.scan_failed());
    let home = scanned(&site);
    assert!(!home.scan_failed());
    assert!(!home.is_scanning());
}
