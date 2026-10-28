//! A slot file the scan cannot read stays on Home as an unreadable slot.

#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Flow, ID_1, Outcomes, Site, all_rows, lock, scanned, slot_bytes, slot_file, slots_of, steam_slot};
use neoffse_app::home::OpenOutcome;
use neoffse_app::{OpenProblem, SlotKind};

const FF1: usize = 0;

/// Slot 1 and slot 2, both valid.
fn two_slots(site: &Site) -> std::path::PathBuf {
    steam_slot(site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    steam_slot(site, FF1, ID_1, slot_file(2), &slot_bytes(2))
}

#[test]
fn an_unopenable_slot_file_is_listed_as_unreadable() {
    let site = Site::new();
    let folder = two_slots(&site);
    let Some(_locked) = lock(&folder.join(slot_file(2))) else { return };
    let home = scanned(&site);

    let slots = slots_of(&all_rows(&home)[0]);
    assert_eq!(slots.len(), 2);
    assert!(slots[0].summarized && !slots[0].unreadable);
    let second = &slots[1];
    assert!(second.unreadable && !second.summarized && second.openable);
    assert_eq!(second.summary.kind, SlotKind::Numbered);
    assert_eq!(second.summary.number, 2);
}

#[test]
fn clicking_it_names_the_cause() {
    let site = Site::new();
    let folder = two_slots(&site);
    let Some(_locked) = lock(&folder.join(slot_file(2))) else { return };
    let flow = Flow::new(&site, None);
    let outcomes = Outcomes::new();

    flow.click(slot_file(2), &outcomes);
    flow.manual.run_all();

    assert_eq!(outcomes.outcomes(), vec![OpenOutcome::Failed(OpenProblem::Unreadable)]);
    assert!(!flow.state.is_open());
}

#[test]
fn an_unopenable_file_that_is_no_slot_gets_no_row() {
    let site = Site::new();
    let folder = steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    std::fs::write(folder.join("stray.txt"), b"x").unwrap();
    let Some(_locked) = lock(&folder.join("stray.txt")) else { return };
    let home = scanned(&site);

    assert_eq!(slots_of(&all_rows(&home)[0]).len(), 1);
    assert_eq!(home.skipped_count(), 0);
}

#[test]
fn the_unreadable_row_is_not_counted_as_skipped() {
    let site = Site::new();
    let folder = two_slots(&site);
    let Some(_locked) = lock(&folder.join(slot_file(2))) else { return };
    let home = scanned(&site);

    assert_eq!(home.skipped_count(), 0);
}
