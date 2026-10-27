//! The discard dialog's "Review & save": the review opens with the leave
//! still pending, a save completes the leave, every other end cancels it, and
//! the window cannot close while a save runs. Saves write into temporary
//! folders under a scripted process probe. The tests that need a close
//! request call `show()` on the testing backend, which has no screen.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Machine, Rig, closeable_rig, editing_site, rig};
use i_slint_backend_testing::AccessibleRole::Button;
use neoffse_app::EditProblem;
use slint::ComponentHandle;
use slint::platform::Key;

fn quiet(site: &common::Site) -> Rig {
    rig(site, Machine::Quiet)
}

/// A dirty save with the discard dialog open from the Back button.
fn back_asked(rig: &Rig) {
    rig.ui.make_dirty();
    rig.ui.window.invoke_back_requested();
    assert!(rig.ui.bridge().get_discard_dialog_open());
    assert!(rig.ui.app.editor.leave_pending());
}

/// The discard dialog to the review, then the review's Save button, run to the end.
fn review_and_save(rig: &Rig) {
    rig.ui.bridge().invoke_discard_review_requested();
    assert!(rig.ui.bridge().get_review_open());
    rig.ui.element("Save", Button).invoke_accessible_default_action();
    rig.ui.manual.run_all();
}

#[test]
fn the_discard_dialog_offers_review_and_save() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    back_asked(&rig);
    assert!(ui.has_element("Review & save", Button));

    ui.bridge().invoke_discard_review_requested();

    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(ui.bridge().get_review_open());
    assert!(ui.app.editor.leave_pending());
    assert_eq!(ui.pending(), 1);
}

#[test]
fn tab_tab_enter_opens_the_review() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    back_asked(&rig);

    ui.key(Key::Tab);
    ui.key(Key::Tab);
    ui.key(Key::Return);

    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(ui.bridge().get_review_open());
    assert!(ui.app.editor.leave_pending());
    assert!(ui.session_open(), "Discard did not fire");
}

#[test]
fn back_then_review_then_save_leaves_for_home() {
    let site = editing_site();
    let rig = quiet(&site);
    let before = rig.bytes();
    back_asked(&rig);

    review_and_save(&rig);

    let ui = &rig.ui;
    assert!(!ui.window.get_editor_open());
    assert!(!ui.session_open());
    assert_ne!(rig.bytes(), before);
    assert_eq!(rig.gil_on_disk(), 500);
    assert_eq!(rig.backups(), 1);
    assert!(!ui.app.editor.leave_pending());
    assert!(!ui.bridge().get_working());
}

#[test]
fn close_then_review_then_save_hides_the_window() {
    let site = editing_site();
    let (rig, _about) = closeable_rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    ui.make_dirty();
    ui.request_close();
    assert!(ui.bridge().get_discard_dialog_open());
    assert!(ui.window.window().is_visible());

    review_and_save(&rig);

    assert!(!ui.window.window().is_visible());
    assert_eq!(rig.gil_on_disk(), 500);
    assert!(!ui.session_open());
}

#[test]
fn cancelling_the_review_cancels_the_leave() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    back_asked(&rig);
    ui.bridge().invoke_discard_review_requested();

    ui.element("Cancel", Button).invoke_accessible_default_action();

    assert!(ui.window.get_editor_open());
    assert!(!ui.bridge().get_review_open());
    assert_eq!(ui.pending(), 1);
    assert!(!ui.app.editor.leave_pending());

    ui.window.invoke_back_requested();
    assert!(ui.bridge().get_discard_dialog_open(), "a new Back asks again");
}

#[test]
fn escape_in_the_review_cancels_the_leave() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    back_asked(&rig);
    ui.bridge().invoke_discard_review_requested();

    ui.key(Key::Escape);

    assert!(!ui.bridge().get_review_open());
    assert!(!ui.app.editor.leave_pending());
    assert!(ui.window.get_editor_open());
}

#[test]
fn a_failed_save_keeps_the_user_in_the_editor() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    rig.break_data_dir();
    let before = rig.bytes();
    back_asked(&rig);

    review_and_save(&rig);

    assert_eq!(ui.bridge().get_notice().problem, EditProblem::SaveFailed);
    assert!(ui.window.get_editor_open());
    assert_eq!(ui.pending(), 1);
    assert!(!ui.app.editor.leave_pending());
    assert_eq!(rig.bytes(), before);
}

#[test]
fn a_running_game_then_save_anyway_leaves() {
    let site = editing_site();
    let rig = rig(&site, Machine::GameRuns);
    let ui = &rig.ui;
    back_asked(&rig);

    review_and_save(&rig);
    assert!(ui.bridge().get_game_running_open());
    assert!(ui.app.editor.leave_pending(), "the leave waits for the answer");
    assert!(ui.window.get_editor_open());

    ui.element("Save anyway", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert!(!ui.window.get_editor_open());
    assert!(!ui.session_open());
    assert_eq!(rig.gil_on_disk(), 500);
    assert!(!ui.app.editor.leave_pending());
}

#[test]
fn a_running_game_then_cancel_stays() {
    let site = editing_site();
    let rig = rig(&site, Machine::GameRuns);
    let ui = &rig.ui;
    let before = rig.bytes();
    back_asked(&rig);
    review_and_save(&rig);
    assert!(ui.bridge().get_game_running_open());

    ui.element("Cancel", Button).invoke_accessible_default_action();

    assert!(!ui.bridge().get_game_running_open());
    assert!(ui.window.get_editor_open());
    assert_eq!(ui.pending(), 1);
    assert!(!ui.app.editor.leave_pending());
    assert_eq!(rig.bytes(), before);
}

#[test]
fn a_changed_file_then_reload_stays_in_the_editor() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    back_asked(&rig);
    rig.change_on_disk(777);

    review_and_save(&rig);
    assert!(ui.bridge().get_changed_on_disk_open());
    assert!(ui.app.editor.leave_pending(), "the leave waits for the answer");

    ui.element("Reload", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert!(ui.window.get_editor_open());
    assert!(!ui.app.editor.leave_pending());
    assert_eq!(ui.pending(), 0);
    assert_eq!(ui.gil().text, "777");
    assert!(!ui.bridge().get_working());
}

#[test]
fn discard_and_cancel_still_work() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    let before = rig.bytes();
    back_asked(&rig);
    ui.element("Cancel", Button).invoke_accessible_default_action();
    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(!ui.app.editor.leave_pending());
    assert!(ui.window.get_editor_open());
    assert_eq!(ui.pending(), 1);

    ui.window.invoke_back_requested();
    ui.element("Discard", Button).invoke_accessible_default_action();

    assert!(!ui.window.get_editor_open());
    assert!(!ui.session_open());
    assert_eq!(rig.bytes(), before);
    assert_eq!(rig.backups(), 0);
}

#[test]
fn close_is_vetoed_while_a_save_runs() {
    let site = editing_site();
    let (rig, _about) = closeable_rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    ui.make_dirty();
    ui.bridge().invoke_save_requested();
    assert!(ui.bridge().get_working());

    ui.request_close();

    assert!(ui.window.window().is_visible());
    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(!ui.app.editor.leave_pending());

    ui.manual.run_all();

    assert_eq!(rig.gil_on_disk(), 500);
    assert!(ui.window.window().is_visible());
    assert!(!ui.bridge().get_working());
    assert!(ui.session_open());
}
