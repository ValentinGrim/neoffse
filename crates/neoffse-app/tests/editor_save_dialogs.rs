//! The two dialogs a save can raise: the running-game warning and the
//! changed-on-disk dialog. Every save writes into temporary folders and runs
//! under a scripted process probe.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::atomic::Ordering;

use common::{Machine, Rig, editing_site, rig, slot_bytes};
use i_slint_backend_testing::AccessibleRole::{Button, Text};
use neoffse_app::{EditProblem, EditorCommand};
use slint::platform::Key;

/// A dirty save (gil "500") whose save met a running game.
fn warned(site: &common::Site) -> Rig {
    let rig = rig(site, Machine::GameRuns);
    rig.ui.make_dirty();
    rig.ui.bridge().invoke_save_requested();
    rig.ui.manual.run_all();
    assert!(rig.ui.bridge().get_game_running_open());
    rig
}

/// A dirty save (gil "500") whose file another program replaced.
fn foreign(site: &common::Site) -> Rig {
    let rig = rig(site, Machine::Quiet);
    rig.ui.make_dirty();
    fs::write(&rig.file, slot_bytes(2)).unwrap();
    rig.ui.bridge().invoke_save_requested();
    rig.ui.manual.run_all();
    assert!(rig.ui.bridge().get_changed_on_disk_open());
    rig
}

#[test]
fn a_running_game_opens_the_warning() {
    let site = editing_site();
    let rig = rig(&site, Machine::GameRuns);
    let ui = &rig.ui;
    let before = rig.bytes();
    ui.make_dirty();

    ui.bridge().invoke_save_requested();
    ui.manual.run_all();

    assert!(ui.bridge().get_game_running_open());
    assert!(ui.bridge().get_blocked());
    assert!(!ui.bridge().get_working());
    assert!(!ui.bridge().get_has_notice());
    assert_eq!(rig.bytes(), before);
    assert_eq!(rig.backups(), 0);
    assert_eq!(ui.pending(), 1);
    assert!(ui.has_element("The game is running. Saving now may be lost or overwritten. Save anyway?", Text));
    assert_eq!(ui.element("Cancel", Button).accessible_enabled(), Some(true));
    assert_eq!(ui.element("Save anyway", Button).accessible_enabled(), Some(true));
}

#[test]
fn escape_cancels_the_warning() {
    let site = editing_site();
    let rig = warned(&site);
    let before = rig.bytes();

    rig.ui.key(Key::Escape);

    assert!(!rig.ui.bridge().get_game_running_open());
    assert_eq!(rig.bytes(), before);
    assert_eq!(rig.ui.pending(), 1);
    assert_eq!(rig.backups(), 0);
}

#[test]
fn enter_cancels_the_warning() {
    let site = editing_site();
    let rig = warned(&site);

    rig.ui.key(Key::Return);

    assert!(!rig.ui.bridge().get_game_running_open());
    assert_eq!(rig.ui.pending(), 1);
    assert_eq!(rig.backups(), 0);
}

#[test]
fn save_anyway_saves_without_asking_again() {
    let site = editing_site();
    let rig = warned(&site);

    rig.ui.element("Save anyway", Button).invoke_accessible_default_action();
    assert!(rig.ui.bridge().get_working());
    assert!(!rig.ui.bridge().get_game_running_open());
    rig.ui.manual.run_all();

    assert_eq!(rig.probes.load(Ordering::SeqCst), 1, "the probe ran for the first try only");
    assert_eq!(rig.gil_on_disk(), 500);
    assert_eq!(rig.ui.bridge().get_notice().problem, EditProblem::Saved);
    assert!(!rig.ui.bridge().get_working());
    assert_eq!(rig.backups(), 1);
}

#[test]
fn a_changed_file_opens_the_reload_dialog() {
    let site = editing_site();
    let rig = foreign(&site);
    let ui = &rig.ui;

    assert!(!ui.bridge().get_working());
    assert!(ui.bridge().get_blocked());
    assert_eq!(rig.bytes(), slot_bytes(2), "the foreign bytes stay");
    assert_eq!(rig.backups(), 0);
    assert!(ui.has_element("The save changed on disk", Text));
    assert_eq!(ui.element("Reload", Button).accessible_enabled(), Some(true));
}

#[test]
fn cancel_keeps_the_edit_and_the_foreign_file() {
    let site = editing_site();
    let rig = foreign(&site);

    rig.ui.element("Cancel", Button).invoke_accessible_default_action();

    assert!(!rig.ui.bridge().get_changed_on_disk_open());
    assert_eq!(rig.ui.pending(), 1);
    assert_eq!(rig.bytes(), slot_bytes(2));
    assert_eq!(rig.ui.gil().text, "500");
}

#[test]
fn escape_cancels_the_reload_dialog() {
    let site = editing_site();
    let rig = foreign(&site);

    rig.ui.key(Key::Escape);

    assert!(!rig.ui.bridge().get_changed_on_disk_open());
    assert_eq!(rig.ui.pending(), 1);
}

#[test]
fn reload_replaces_the_session_with_the_disk() {
    let site = editing_site();
    let rig = rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    ui.navigate(EditorCommand::Party, 0);
    ui.make_dirty();
    rig.change_on_disk(777);
    ui.bridge().invoke_save_requested();
    ui.manual.run_all();
    assert!(ui.bridge().get_changed_on_disk_open());

    ui.element("Reload", Button).invoke_accessible_default_action();
    assert!(ui.bridge().get_working());
    assert!(!ui.bridge().get_changed_on_disk_open());
    ui.manual.run_all();

    assert_eq!(ui.pending(), 0);
    assert_eq!(ui.gil().text, "777");
    assert!(!ui.has_element("Review & save", Button));
    assert!(ui.window.get_editor_open());
    assert_eq!(ui.bridge().get_command(), EditorCommand::Party);
    assert_eq!(ui.bridge().get_character(), 0);
    assert!(!ui.bridge().get_has_notice());
    assert!(!ui.bridge().get_working());
}

#[test]
fn a_failed_reload_keeps_the_edit() {
    let site = editing_site();
    let rig = foreign(&site);
    let ui = &rig.ui;
    fs::remove_file(&rig.file).unwrap();

    ui.element("Reload", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert_eq!(ui.bridge().get_notice().problem, EditProblem::ReloadFailed);
    assert_eq!(ui.pending(), 1);
    assert_eq!(ui.gil().text, "500");
    assert!(!ui.bridge().get_working());
    assert!(ui.window.get_editor_open());
    assert!(ui.has_element("The save could not be reloaded. Your changes are still open.", Text));
}
