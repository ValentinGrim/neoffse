//! The discard dialog while a worker runs: Discard and Review & save do
//! nothing, and work again once the worker ends. A held save owns the session
//! lock, so these tests read only bridge flags and the app state while it
//! runs: `pending()` and the view model's `history()` would wait for it.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Machine, editing_site, rig};

#[test]
fn discard_does_nothing_while_a_save_runs() {
    let site = editing_site();
    let rig = rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    ui.ask_discard();
    let held = rig.hold_save();

    ui.bridge().invoke_discard_confirmed();

    assert!(ui.bridge().get_discard_dialog_open());
    assert!(ui.window.get_editor_open());
    assert!(ui.session_open());
    assert!(ui.app.editor.leave_pending());

    held.release();
    ui.manual.run_all();
    assert_eq!(rig.gil_on_disk(), 500);
}

#[test]
fn review_and_save_does_nothing_while_a_save_runs() {
    let site = editing_site();
    let rig = rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    ui.ask_discard();
    let held = rig.hold_save();

    ui.bridge().invoke_discard_review_requested();

    assert!(ui.bridge().get_discard_dialog_open());
    assert!(!ui.bridge().get_review_open());
    assert!(ui.app.editor.leave_pending());
    assert!(!ui.bridge().get_has_notice());

    held.release();
    ui.manual.run_all();
    assert_eq!(rig.gil_on_disk(), 500);
}

#[test]
fn discard_does_nothing_while_a_restore_runs() {
    let site = editing_site();
    let rig = rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    ui.ask_discard();
    ui.restore_bridge().set_working(true);

    ui.bridge().invoke_discard_confirmed();

    assert!(ui.bridge().get_discard_dialog_open());
    assert!(ui.window.get_editor_open());
    assert!(ui.session_open());
    assert!(ui.app.editor.leave_pending());

    ui.restore_bridge().set_working(false);
    ui.bridge().invoke_discard_confirmed();
    assert!(!ui.window.get_editor_open());
    assert!(!ui.session_open());
}

#[test]
fn review_and_save_does_nothing_while_a_restore_runs() {
    let site = editing_site();
    let rig = rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    ui.ask_discard();
    ui.restore_bridge().set_working(true);

    ui.bridge().invoke_discard_review_requested();

    assert!(ui.bridge().get_discard_dialog_open());
    assert!(!ui.bridge().get_review_open());
    assert!(ui.app.editor.leave_pending());

    ui.restore_bridge().set_working(false);
    ui.bridge().invoke_discard_review_requested();
    assert!(ui.bridge().get_review_open());
    assert!(!ui.bridge().get_discard_dialog_open());
}
