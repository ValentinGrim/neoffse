//! The restore and the open editor session: a restore that replaces the open
//! file reopens it, after the discard question when the session is dirty. The
//! probe is scripted, every write goes to temporary folders, and the one test
//! that needs a close request calls `show()` on the testing backend, which
//! has no screen.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::Arc;

use common::{Machine, Rig, Site, closeable_rig, editing_site, make_backup, rig, slot_bytes, slot_file};
use i_slint_backend_testing::AccessibleRole::Button;
use neoffse_app::{EditorCommand, RestoreProblem};
use slint::ComponentHandle;

/// The editor open on slot 1, a backup of the file as it is, then another
/// program saves gil 777 into it. The open session is stale but clean.
fn stale(machine: Machine) -> (Site, Rig, i64) {
    let site = editing_site();
    let rig = rig(&site, machine);
    let original = rig.gil_on_disk();
    rig.backup_now("before-save");
    rig.change_on_disk(777);
    (site, rig, original)
}

/// The gil the editor page shows.
fn shown(rig: &Rig) -> String {
    rig.ui.gil().text.to_string()
}

fn notice(rig: &Rig) -> RestoreProblem {
    rig.ui.restore_bridge().get_notice().problem
}

/// The discard dialog opened over the page by Restore, on a dirty session.
fn asked(rig: &Rig) {
    rig.ui.make_dirty();
    rig.open_page();
    rig.start_restore();
    assert!(rig.ui.bridge().get_discard_dialog_open());
}

#[test]
fn a_clean_session_is_reopened_after_the_restore() {
    let (_site, rig, original) = stale(Machine::Quiet);
    let ui = &rig.ui;
    let before = ui.app.state.session().unwrap();
    let summary = ui.window.get_editor_summary();
    rig.open_page();

    rig.start_restore();
    assert!(!ui.bridge().get_discard_dialog_open(), "a clean session asks nothing");
    assert!(ui.restore_bridge().get_working());
    ui.manual.run_all();

    assert_eq!(rig.gil_on_disk(), original);
    assert_ne!(original, 777);
    assert_eq!(shown(&rig), original.to_string());
    assert_eq!(ui.pending(), 0);
    assert!(!Arc::ptr_eq(&before, &ui.app.state.session().unwrap()), "the session was reopened");
    let after = ui.window.get_editor_summary();
    assert_eq!((after.kind, after.number), (summary.kind, summary.number));
    assert!(ui.window.get_editor_open() && ui.window.get_restore_open());
    assert_eq!(notice(&rig), RestoreProblem::Restored);
    assert!(!ui.restore_bridge().get_working());

    ui.restore_bridge().invoke_back_requested();
    assert!(!ui.window.get_restore_open());
    assert!(ui.window.get_editor_open());
    assert_eq!(shown(&rig), original.to_string());
}

#[test]
fn the_navigation_survives_the_reopen() {
    let (_site, rig, _) = stale(Machine::Quiet);
    let ui = &rig.ui;
    ui.navigate(EditorCommand::Items, 0);
    rig.open_page();
    rig.start_restore();
    ui.manual.run_all();
    assert_eq!(notice(&rig), RestoreProblem::Restored);

    ui.restore_bridge().invoke_back_requested();

    assert_eq!(ui.bridge().get_command(), EditorCommand::Items);
    assert_eq!(ui.bridge().get_character(), 0);
}

#[test]
fn another_slot_leaves_the_session_alone() {
    let site = editing_site();
    let rig = rig(&site, Machine::Quiet);
    let folder = rig.file.parent().unwrap();
    fs::write(folder.join(slot_file(2)), slot_bytes(2)).unwrap();
    make_backup(&rig.data_dir(), "ff1-pr", folder, &[slot_file(2)], "before-save", common::T0, 0);
    let before = rig.ui.app.state.session().unwrap();
    rig.open_page();

    rig.start_restore();
    // The restore, then the Home scan it starts; no reopen follows.
    rig.ui.manual.run_all();

    assert_eq!(notice(&rig), RestoreProblem::Restored);
    assert!(Arc::ptr_eq(&before, &rig.ui.app.state.session().unwrap()));
    assert!(rig.ui.window.get_editor_open());
}

#[test]
fn the_same_file_name_in_another_folder_leaves_the_session_alone() {
    let site = editing_site();
    let rig = rig(&site, Machine::Quiet);
    let elsewhere = tempfile::tempdir().unwrap();
    fs::write(elsewhere.path().join(slot_file(1)), slot_bytes(2)).unwrap();
    make_backup(&rig.data_dir(), "ff1-pr", elsewhere.path(), &[slot_file(1)], "before-save", common::T0, 0);
    fs::write(elsewhere.path().join(slot_file(1)), slot_bytes(1)).unwrap();
    let opened = rig.bytes();
    let before = rig.ui.app.state.session().unwrap();
    rig.open_page();

    rig.start_restore();
    // The restore, then the Home scan it starts; no reopen follows.
    rig.ui.manual.run_all();

    assert_eq!(notice(&rig), RestoreProblem::Restored);
    assert_eq!(fs::read(elsewhere.path().join(slot_file(1))).unwrap(), slot_bytes(2));
    assert_eq!(rig.bytes(), opened, "the open file was not touched");
    assert!(Arc::ptr_eq(&before, &rig.ui.app.state.session().unwrap()));
}

#[test]
fn a_dirty_session_asks_before_the_restore() {
    let (_site, rig, _) = stale(Machine::Quiet);
    let ui = &rig.ui;
    let before = rig.bytes();

    asked(&rig);

    assert!(ui.app.editor.leave_pending());
    assert!(!ui.restore_bridge().get_confirm_open());
    assert!(!ui.restore_bridge().get_working());
    assert!(ui.restore_bridge().get_blocked(), "the dialog keeps the page blocked");
    assert_eq!(ui.manual.queued_jobs(), 0);
    assert_eq!(rig.bytes(), before);
    assert_eq!(ui.pending(), 1);
    assert!(ui.session_open());
}

#[test]
fn cancel_in_the_discard_dialog_keeps_everything() {
    let (_site, rig, _) = stale(Machine::Quiet);
    let ui = &rig.ui;
    let before = rig.bytes();
    asked(&rig);

    ui.element("Cancel", Button).invoke_accessible_default_action();

    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(!ui.app.editor.leave_pending());
    assert!(ui.window.get_restore_open());
    assert_eq!(ui.pending(), 1);
    assert_eq!(rig.bytes(), before);
    assert!(ui.app.restore.selected().is_some(), "the selection stays");
    ui.element("Restore the backup of 2020-09-13 12:26 UTC", Button)
        .invoke_accessible_default_action();
    assert!(ui.restore_bridge().get_confirm_open(), "a new Restore click works");
}

#[test]
fn discard_restores_and_replaces_the_session() {
    let (_site, rig, original) = stale(Machine::Quiet);
    let ui = &rig.ui;
    let before = ui.app.state.session().unwrap();
    asked(&rig);

    ui.element("Discard", Button).invoke_accessible_default_action();
    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(ui.restore_bridge().get_working());
    assert!(ui.session_open(), "the session stays until the file is replaced");
    ui.manual.run_all();

    assert_eq!(rig.gil_on_disk(), original);
    assert_eq!(ui.pending(), 0);
    assert_eq!(shown(&rig), original.to_string());
    assert!(!Arc::ptr_eq(&before, &ui.app.state.session().unwrap()));
    assert_eq!(notice(&rig), RestoreProblem::Restored);
    assert!(!ui.app.editor.leave_pending());
    assert!(ui.window.get_editor_open() && ui.window.get_restore_open());
}

#[test]
fn a_restore_that_cannot_start_keeps_the_edits_after_discard() {
    let (_site, rig, _) = stale(Machine::GameRuns);
    let ui = &rig.ui;
    let before = rig.bytes();
    asked(&rig);

    ui.element("Discard", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert!(ui.restore_bridge().get_game_running_open());
    assert_eq!(ui.pending(), 1, "the edits are still in the session");
    assert_eq!(rig.bytes(), before);

    ui.element("Cancel", Button).invoke_accessible_default_action();

    assert!(!ui.restore_bridge().get_game_running_open());
    assert_eq!(ui.pending(), 1);
    assert!(ui.session_open());
    assert_eq!(rig.bytes(), before);
    assert!(!ui.app.editor.leave_pending());
}

#[test]
fn restore_anyway_then_reopens() {
    let (_site, rig, original) = stale(Machine::GameRuns);
    let ui = &rig.ui;
    let before = ui.app.state.session().unwrap();
    asked(&rig);
    ui.element("Discard", Button).invoke_accessible_default_action();
    ui.manual.run_all();
    assert!(ui.restore_bridge().get_game_running_open());

    ui.element("Restore anyway", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert_eq!(ui.pending(), 0);
    assert_eq!(rig.gil_on_disk(), original);
    assert_eq!(shown(&rig), original.to_string());
    assert!(!Arc::ptr_eq(&before, &ui.app.state.session().unwrap()));
    assert_eq!(notice(&rig), RestoreProblem::Restored);
}

#[test]
fn a_reopen_that_fails_closes_the_editor() {
    let site = editing_site();
    let rig = rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    let valid = rig.bytes();
    let garbage = vec![0xA5_u8; 64];
    fs::write(&rig.file, &garbage).unwrap();
    rig.backup_now("before-save");
    fs::write(&rig.file, &valid).unwrap();
    rig.open_page();

    rig.start_restore();
    ui.manual.run_all();

    assert_eq!(rig.bytes(), garbage, "the restore is byte for byte");
    assert_eq!(notice(&rig), RestoreProblem::ReopenFailed);
    assert!(!ui.app.state.is_open());
    assert!(!ui.window.get_editor_open());
    assert!(ui.window.get_restore_open());
    assert!(!ui.restore_bridge().get_working());

    ui.restore_bridge().invoke_back_requested();

    assert!(!ui.window.get_restore_open());
    assert!(!ui.window.get_editor_open());
    ui.manual.run_all();
    assert!(!ui.restore_bridge().get_working());
}

#[test]
fn a_close_request_replaces_the_pending_restore() {
    let site = editing_site();
    let (rig, _about) = closeable_rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    let before = rig.bytes();
    rig.backup_now("before-save");
    asked(&rig);

    ui.request_close();
    assert!(ui.bridge().get_discard_dialog_open());
    assert!(ui.window.window().is_visible());

    ui.element("Discard", Button).invoke_accessible_default_action();

    assert!(!ui.window.window().is_visible(), "Discard closes the window");
    assert_eq!(ui.manual.queued_jobs(), 0, "no restore started");
    assert_eq!(rig.bytes(), before);
    assert_eq!(rig.backups(), 1, "no safety backup");
    assert!(!ui.session_open());
}
