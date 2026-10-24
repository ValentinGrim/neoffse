//! "Review & save" in front of a restore: the save ends the pending leave,
//! the restore then replaces the file the save wrote, and every other end of
//! the save flow cancels the restore. Saves and restores write into
//! temporary folders under a scripted process probe.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::atomic::Ordering;

use common::{Machine, Rig, Site, editing_site, rig, slot_bytes, slot_file};
use i_slint_backend_testing::AccessibleRole::Button;
use neoffse::core::Game;
use neoffse_app::RestoreProblem;

/// The editor open on slot 1 with the backup `slot_bytes(2)` in the data
/// folder, the file back to `slot_bytes(1)`, one gil edit pending, the page
/// open and the discard dialog asking.
fn asked(machine: Machine) -> (Site, Rig) {
    let site = editing_site();
    let rig = rig(&site, machine);
    fs::write(&rig.file, slot_bytes(2)).unwrap();
    rig.backup_now("before-save");
    fs::write(&rig.file, slot_bytes(1)).unwrap();
    rig.ui.make_dirty();
    rig.open_page();
    rig.start_restore();
    assert!(rig.ui.bridge().get_discard_dialog_open());
    (site, rig)
}

fn notice(rig: &Rig) -> RestoreProblem {
    rig.ui.restore_bridge().get_notice().problem
}

fn reasons(rig: &Rig) -> Vec<String> {
    common::listing(&rig.data_dir(), Game::Ff1Pr)
        .into_iter()
        .map(|backup| backup.manifest.reason)
        .collect()
}

/// The discard dialog to the review, with the leave still pending.
fn review(rig: &Rig) {
    rig.ui.bridge().invoke_discard_review_requested();
    assert!(rig.ui.bridge().get_review_open());
    assert!(rig.ui.app.editor.leave_pending());
}

#[test]
fn review_and_save_then_restore() {
    let (_site, rig) = asked(Machine::Quiet);
    let ui = &rig.ui;
    let before = ui.app.state.session().unwrap();
    review(&rig);

    ui.element("Save", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert_eq!(rig.bytes(), slot_bytes(2), "the file equals the backup");
    assert_eq!(reasons(&rig), ["before-restore", "before-save", "before-save"]);
    let backups = common::listing(&rig.data_dir(), Game::Ff1Pr);
    let kept = fs::read(backups[0].dir.join(slot_file(1))).unwrap();
    assert_ne!(kept, slot_bytes(2), "the safety backup is not the restored bytes");
    assert_ne!(kept, slot_bytes(1), "the safety backup holds the saved edit");
    assert_eq!(
        fs::read(backups[1].dir.join(slot_file(1))).unwrap(),
        slot_bytes(1),
        "the save backed up the original"
    );

    assert_eq!(ui.pending(), 0);
    assert!(!std::sync::Arc::ptr_eq(&before, &ui.app.state.session().unwrap()), "the session was reopened");
    assert!(!ui.app.editor.leave_pending());
    assert_eq!(notice(&rig), RestoreProblem::Restored);
    assert!(ui.window.get_editor_open() && ui.window.get_restore_open());
}

#[test]
fn cancelling_the_review_cancels_the_restore() {
    let (_site, rig) = asked(Machine::Quiet);
    let ui = &rig.ui;
    review(&rig);

    ui.element("Cancel", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert_eq!(rig.bytes(), slot_bytes(1));
    assert_eq!(ui.pending(), 1);
    assert!(!ui.app.editor.leave_pending());
    assert!(ui.window.get_restore_open() && ui.window.get_editor_open());
    assert!(!ui.restore_bridge().get_working());
    assert_eq!(reasons(&rig), ["before-save"]);
}

/// A save over a file another program changed ends in the reload dialog,
/// with the leave still pending.
fn changed(rig: &Rig) {
    rig.change_on_disk(777);
    review(rig);
    rig.ui.element("Save", Button).invoke_accessible_default_action();
    rig.ui.manual.run_all();
    assert!(rig.ui.bridge().get_changed_on_disk_open());
    assert!(rig.ui.app.editor.leave_pending());
}

#[test]
fn a_changed_file_then_cancel_restores_nothing() {
    let (_site, rig) = asked(Machine::Quiet);
    let ui = &rig.ui;
    changed(&rig);

    ui.element("Cancel", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert_eq!(rig.gil_on_disk(), 777, "the file stays as the other program wrote it");
    assert!(!ui.app.editor.leave_pending());
    assert_eq!(ui.pending(), 1);
    assert_eq!(reasons(&rig), ["before-save"], "no safety backup");
    assert!(ui.window.get_restore_open() && ui.window.get_editor_open());
    assert!(!ui.restore_bridge().get_working());
}

#[test]
fn a_changed_file_then_reload_restores_nothing() {
    let (_site, rig) = asked(Machine::Quiet);
    let ui = &rig.ui;
    changed(&rig);

    ui.element("Reload", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert_eq!(rig.gil_on_disk(), 777);
    assert!(!ui.app.editor.leave_pending());
    assert_eq!(ui.pending(), 0, "the reload dropped the edits");
    assert_eq!(ui.gil().text, "777");
    assert_eq!(reasons(&rig), ["before-save"]);
    assert!(ui.window.get_restore_open() && ui.window.get_editor_open());
    assert!(!ui.restore_bridge().get_working());
}

#[test]
fn a_running_game_asks_twice() {
    let (_site, rig) = asked(Machine::GameRuns);
    let ui = &rig.ui;
    review(&rig);

    ui.element("Save", Button).invoke_accessible_default_action();
    ui.manual.run_all();
    assert!(ui.bridge().get_game_running_open(), "the save warns first");
    assert_eq!(rig.bytes(), slot_bytes(1));

    ui.element("Save anyway", Button).invoke_accessible_default_action();
    ui.manual.run_all();
    assert!(!ui.bridge().get_game_running_open());
    assert!(ui.restore_bridge().get_game_running_open(), "then the restore warns");
    assert_ne!(rig.bytes(), slot_bytes(2), "the restore has not run");

    ui.element("Restore anyway", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert_eq!(rig.bytes(), slot_bytes(2));
    assert_eq!(notice(&rig), RestoreProblem::Restored);
    assert_eq!(reasons(&rig)[0], "before-restore");
    assert_eq!(rig.probes.load(Ordering::SeqCst), 2, "one probe for the save, one for the restore");
    assert_eq!(ui.pending(), 0);
}
