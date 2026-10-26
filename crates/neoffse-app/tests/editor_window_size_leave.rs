//! The window returns to the size Home had on the other ways out of the
//! editor: a discard, a save that ends a pending leave, and a restore whose
//! reopen fails. A cancelled discard and a restore that reopens the editor
//! keep the editor's size. Sizes go through `set_size` and read back in
//! logical pixels on Slint's testing backend; no window shows.
//!
//! The backend stores a size as given. A real window manager may refuse,
//! clamp or delay it, so these tests prove the app asks for the right size
//! and nothing about the answer.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{Machine, Rig, Site, Ui, editing_site, resize, rig};
use i_slint_backend_testing::AccessibleRole::Button;
use neoffse_app::RestoreProblem;
use slint::ComponentHandle;

/// The window size in logical pixels.
fn logical(ui: &Ui) -> (f32, f32) {
    let size = ui.window.window().size().to_logical(ui.window.window().scale_factor());
    (size.width, size.height)
}

fn assert_size(ui: &Ui, expected: (f32, f32), what: &str) {
    let (width, height) = logical(ui);
    assert!(
        (width - expected.0).abs() < 0.5 && (height - expected.1).abs() < 0.5,
        "{what}: {width}x{height}, expected {expected:?}"
    );
}

/// Home at 900 by 650, the first slot open and dirty, the window resized to
/// 1100 by 700, Back pressed: the discard dialog asks.
fn asked(site: &Site) -> Ui {
    let ui = Ui::scanned(site, None);
    resize(&ui, 900.0, 650.0);
    ui.window.invoke_slot_clicked(ui.first_slots()[0].key);
    ui.manual.run_all();
    assert!(ui.window.get_editor_open(), "the save opens");
    ui.make_dirty();
    resize(&ui, 1100.0, 700.0);
    ui.window.invoke_back_requested();
    assert!(ui.bridge().get_discard_dialog_open(), "the discard dialog asks");
    ui
}

#[test]
fn a_discard_restores_the_home_size() {
    let site = editing_site();
    let ui = asked(&site);
    ui.bridge().invoke_discard_confirmed();
    assert!(!ui.window.get_editor_open());
    assert_size(&ui, (900.0, 650.0), "the discard restores Home's size");
}

#[test]
fn a_cancelled_discard_keeps_the_editor_size() {
    let site = editing_site();
    let ui = asked(&site);
    ui.bridge().invoke_discard_cancelled();
    assert!(ui.window.get_editor_open());
    assert_size(&ui, (1100.0, 700.0), "the dialog alone restores nothing");
}

#[test]
fn a_save_that_ends_a_leave_restores_the_size() {
    let site = editing_site();
    let rig = rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    let home = logical(ui);
    ui.make_dirty();
    resize(ui, home.0 + 300.0, home.1 + 100.0);
    ui.window.invoke_back_requested();
    assert!(ui.bridge().get_discard_dialog_open());

    ui.bridge().invoke_discard_review_requested();
    ui.element("Save", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert!(!ui.window.get_editor_open());
    assert_eq!(rig.gil_on_disk(), 500, "the save ran");
    assert_size(ui, home, "the save that ends the leave restores Home's size");
}

/// A backup of garbage taken while the file holds it, the file valid again,
/// the Restore page open and the restore started.
fn restore_garbage(rig: &Rig) {
    let valid = rig.bytes();
    fs::write(&rig.file, vec![0xA5_u8; 64]).unwrap();
    rig.backup_now("before-save");
    fs::write(&rig.file, &valid).unwrap();
    rig.open_page();
    rig.start_restore();
}

#[test]
fn a_reopen_failure_restores_the_home_size() {
    let site = editing_site();
    let rig = rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    let home = logical(ui);
    restore_garbage(&rig);
    resize(ui, home.0 + 300.0, home.1 + 100.0);

    ui.manual.run_all();

    assert_eq!(ui.restore_bridge().get_notice().problem, RestoreProblem::ReopenFailed);
    assert!(!ui.window.get_editor_open());
    assert!(ui.window.get_restore_open(), "the Restore page stays");
    assert_size(ui, home, "the failed reopen restores Home's size");
}

#[test]
fn a_restore_that_succeeds_keeps_the_editor_size() {
    let site = editing_site();
    let rig = rig(&site, Machine::Quiet);
    let ui = &rig.ui;
    let home = logical(ui);
    rig.backup_now("before-save");
    rig.change_on_disk(777);
    rig.open_page();
    let editor = (home.0 + 300.0, home.1 + 100.0);
    resize(ui, editor.0, editor.1);

    rig.start_restore();
    ui.manual.run_all();

    assert_eq!(ui.restore_bridge().get_notice().problem, RestoreProblem::Restored);
    assert!(ui.window.get_editor_open() && ui.window.get_restore_open());
    assert_size(ui, editor, "a reopen that works restores nothing");
}
