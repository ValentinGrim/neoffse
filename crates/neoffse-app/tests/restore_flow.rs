//! The restore flow from the page: the confirmation, the run on a worker,
//! the notices and the refresh. Everything runs on temporary folders under a
//! scripted process probe. The sentences of a dialog and of a notice are read
//! through the accessibility tree where it exposes them; the notice data comes
//! from the bridge.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{Paged, Site, T0, editing_site, install, listing, make_backup, restore_page, slot_bytes, slot_file};
use i_slint_backend_testing::AccessibleRole::{Button, Text};
use neoffse::core::Game;
use neoffse_app::editor::stamp_of;
use neoffse_app::{BackupReason, RestoreProblem};
use slint::platform::{Key, PointerEventButton};

/// The stamp of a backup made at `T0`.
const STAMP: &str = "2020-09-13 12:26 UTC";

/// The label of the row button of the backup at `T0`.
const ROW_BUTTON: &str = "Restore the backup of 2020-09-13 12:26 UTC";

/// `restore_page()` with the confirmation open.
fn asked() -> (Site, Paged) {
    let (site, p) = restore_page();
    p.ui.element(ROW_BUTTON, Button).invoke_accessible_default_action();
    (site, p)
}

#[test]
fn a_restore_asks_first() {
    let (_site, p) = restore_page();
    p.ui.element(ROW_BUTTON, Button).invoke_accessible_default_action();

    let bridge = p.bridge();
    assert!(bridge.get_confirm_open());
    assert_eq!(bridge.get_pending_stamp().as_str(), STAMP);
    assert!(bridge.get_blocked());
    assert!(!bridge.get_working());
    assert_eq!(p.file(), slot_bytes(2));
    assert_eq!(p.ui.manual.queued_jobs(), 0);
    assert!(p.ui.has_element("Restore the backup of 2020-09-13 12:26 UTC? Your current files are backed up first.", Text));
    assert_eq!(p.ui.element("Cancel", Button).accessible_enabled(), Some(true));
}

/// The dialog closes, nothing is written, and the page has the focus again:
/// Escape then goes back.
fn assert_cancelled(p: &Paged) {
    assert!(!p.bridge().get_confirm_open());
    assert!(!p.bridge().get_working());
    assert_eq!(p.file(), slot_bytes(2));
    assert_eq!(p.ui.manual.queued_jobs(), 0);
    assert_eq!(common::listing(&p.data_dir(), Game::Ff1Pr).len(), 1);
    assert!(p.ui.window.get_restore_open());
    p.ui.key(Key::Escape);
    assert!(!p.ui.window.get_restore_open(), "the page took the focus back");
}

#[test]
fn the_cancel_button_cancels_the_confirmation() {
    let (_site, p) = asked();
    // A click, not an accessible action: Slint evaluates the `changed` handler
    // that gives the page its focus back after an input event.
    p.ui.element("Cancel", Button).mock_single_click(PointerEventButton::Left);
    assert_cancelled(&p);
}

#[test]
fn escape_cancels_the_confirmation() {
    let (_site, p) = asked();
    p.ui.key(Key::Escape);
    assert_cancelled(&p);
}

#[test]
fn enter_cancels_the_confirmation() {
    let (_site, p) = asked();
    p.ui.key(Key::Return);
    assert_cancelled(&p);
}

#[test]
fn confirming_restores_on_a_worker_and_blocks_the_page() {
    let (_site, p) = restore_page();
    p.start();

    let bridge = p.bridge();
    assert!(bridge.get_working());
    assert!(!bridge.get_confirm_open());
    assert!(bridge.get_blocked());
    assert_eq!(p.file(), slot_bytes(2), "nothing is written before the worker runs");
    assert_eq!(p.ui.element("Back", Button).accessible_enabled(), Some(false));
    assert_eq!(p.ui.element(ROW_BUTTON, Button).accessible_enabled(), Some(false));
    assert!(p.ui.has_element("Restoring...", Text));

    p.ui.key(Key::Escape);
    bridge.invoke_back_requested();
    assert!(p.ui.window.get_restore_open());
    bridge.invoke_restore_requested(0);
    assert!(!bridge.get_confirm_open(), "a second request is ignored");
    assert_eq!(p.ui.manual.queued_jobs(), 1);

    p.ui.manual.run_all();
    assert_eq!(p.file(), slot_bytes(1));
}

#[test]
fn the_restore_ends_in_an_info_notice_with_the_safety_date() {
    let (_site, p) = restore_page();
    p.start();
    p.ui.manual.run_all();

    assert_eq!(p.file(), slot_bytes(1));
    let newest = listing(&p.data_dir(), Game::Ff1Pr).remove(0);
    assert_eq!(newest.manifest.reason, "before-restore");
    let safety = stamp_of(&newest.manifest.created).unwrap();
    let bridge = p.bridge();
    assert!(bridge.get_has_notice());
    assert_eq!(bridge.get_notice().problem, RestoreProblem::Restored);
    assert_eq!(bridge.get_notice().value.as_str(), safety);
    assert!(!bridge.get_working());
    assert!(!bridge.get_blocked());
    let rows = p.rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].reason, BackupReason::BeforeRestore);
    assert_eq!(rows[0].stamp.as_str(), safety);
    assert!(p.ui.has_element(&format!("Restored. Previous files kept: {safety}"), Text));
}

#[test]
fn undoing_a_restore_restores_the_before_restore_backup() {
    let (_site, p) = restore_page();
    p.start();
    p.ui.manual.run_all();
    assert_eq!(p.file(), slot_bytes(1));

    let safety = p.rows()[0].stamp.to_string();
    p.ui.element(&format!("Restore the backup of {safety}"), Button)
        .invoke_accessible_default_action();
    p.ui.element("Restore", Button).invoke_accessible_default_action();
    p.ui.manual.run_all();

    assert_eq!(p.file(), slot_bytes(2));
    assert_eq!(p.bridge().get_notice().problem, RestoreProblem::Restored);
}

#[test]
fn a_restore_with_nothing_to_back_up_says_plain_restored() {
    let (_site, p) = restore_page();
    fs::remove_file(p.folder.join(slot_file(1))).unwrap();
    p.start();
    p.ui.manual.run_all();

    assert_eq!(p.file(), slot_bytes(1));
    assert_eq!(p.bridge().get_notice().problem, RestoreProblem::Restored);
    assert_eq!(p.bridge().get_notice().value.as_str(), "");
    assert_eq!(p.rows().len(), 1);
    assert!(p.ui.has_element("Restored.", Text));
}

#[test]
fn a_save_that_no_longer_opens_is_repaired() {
    let site = editing_site();
    let p = Paged::home(&site);
    p.backup("before-save", 0);
    fs::write(p.folder.join(slot_file(1)), [9_u8; 64]).unwrap();
    p.ui.window.invoke_rescan_requested();
    p.ui.manual.run_all();
    assert!(p.ui.first_slots()[0].unreadable);

    p.open_from_home();
    p.start();
    p.ui.manual.run_all();
    p.bridge().invoke_back_requested();

    assert!(!p.ui.window.get_restore_open());
    let slot = &p.ui.first_slots()[0];
    assert!(slot.summarized && !slot.unreadable, "the scan after the restore read the slot");
}

#[test]
fn an_unsupported_game_restores_from_home() {
    let site = editing_site();
    install(&site, 1);
    let p = Paged::home(&site);
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("notes.txt"), [7_u8; 16]).unwrap();
    make_backup(&p.data_dir(), "ff2-pr", source.path(), &["notes.txt"], "before-save", T0, 0);
    fs::write(source.path().join("notes.txt"), [8_u8; 4]).unwrap();

    p.ui.element("Backups of Final Fantasy II", Button).invoke_accessible_default_action();
    p.start();
    p.ui.manual.run_all();

    assert_eq!(fs::read(source.path().join("notes.txt")).unwrap(), [7_u8; 16]);
    assert_eq!(p.bridge().get_notice().problem, RestoreProblem::Restored);
    assert_eq!(listing(&p.data_dir(), Game::Ff2Pr).len(), 2);
}
