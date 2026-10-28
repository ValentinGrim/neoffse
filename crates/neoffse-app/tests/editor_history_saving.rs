//! Undo and redo while a save runs. A held save owns the session lock, so
//! these tests read only bridge flags while it runs: `pending()` and the view
//! model's `history()` would wait for it.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Machine, Rig, editing_site, rig};

/// A rig with gil 500 then 600 committed and one undo done: one change
/// pending, a redo available.
fn undone(site: &common::Site) -> Rig {
    let rig = rig(site, Machine::Quiet);
    let ui = &rig.ui;
    ui.make_dirty();
    ui.commit(ui.gil().key, "600");
    ui.bridge().invoke_undo_requested();
    assert_eq!(ui.bridge().get_pending_count(), 1);
    assert!(ui.bridge().get_can_redo());
    rig
}

fn assert_untouched(rig: &Rig) {
    let bridge = rig.ui.bridge();
    assert_eq!(bridge.get_pending_count(), 1);
    assert!(bridge.get_can_undo());
    assert!(bridge.get_can_redo());
    assert!(!bridge.get_has_notice());
}

#[test]
fn undo_does_nothing_while_a_save_runs() {
    let site = editing_site();
    let rig = undone(&site);
    let held = rig.hold_save();

    rig.ui.bridge().invoke_undo_requested();

    assert_untouched(&rig);
    held.release();
    rig.ui.manual.run_all();
    assert_eq!(rig.gil_on_disk(), 500);
    assert_eq!(rig.ui.bridge().get_pending_count(), 0);
}

#[test]
fn redo_does_nothing_while_a_save_runs() {
    let site = editing_site();
    let rig = undone(&site);
    let held = rig.hold_save();

    rig.ui.bridge().invoke_redo_requested();

    assert_untouched(&rig);
    held.release();
    rig.ui.manual.run_all();
    assert_eq!(rig.gil_on_disk(), 500);
    assert_eq!(rig.ui.bridge().get_pending_count(), 0);
}
