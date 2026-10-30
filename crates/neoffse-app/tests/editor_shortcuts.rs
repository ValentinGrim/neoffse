//! Ctrl+Z, Ctrl+Y and Ctrl+Shift+Z on the editor page, and the chords a
//! focused text box keeps. Key events go through `Window::dispatch_event` on
//! Slint's testing backend; no window shows and no event loop runs.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Ui, editing_site, open_editor};
use i_slint_backend_testing::AccessibleRole::Button;
use neoffse_app::EditorCommand;
use slint::platform::Key;

/// The chord that redoes inside a text box on this platform.
fn box_redo(ui: &Ui) {
    if cfg!(windows) {
        ui.chord(&[Key::Control], "y");
    } else {
        ui.chord(&[Key::Control, Key::Shift], "Z");
    }
}

/// The redo chord a text box ignores on this platform.
fn page_redo(ui: &Ui) {
    if cfg!(windows) {
        ui.chord(&[Key::Control, Key::Shift], "Z");
    } else {
        ui.chord(&[Key::Control], "y");
    }
}

/// A dirty save (gil "500") undone once, with the Level box of Aria focused.
fn undone_with_a_box_focused(ui: &Ui) {
    ui.make_dirty();
    ui.bridge().invoke_undo_requested();
    assert_eq!(ui.pending(), 0);
    assert!(ui.bridge().get_can_redo());
    ui.navigate(EditorCommand::Party, 0);
    ui.focus_input("Level");
}

#[test]
fn ctrl_z_undoes() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.chord(&[Key::Control], "z");
    assert_eq!(ui.pending(), 0);
    assert_eq!(ui.gil().text, "123456");
}

#[test]
fn ctrl_y_redoes() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.bridge().invoke_undo_requested();
    ui.chord(&[Key::Control], "y");
    assert_eq!(ui.pending(), 1);
    assert_eq!(ui.gil().text, "500");
}

#[test]
fn ctrl_shift_z_redoes() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.bridge().invoke_undo_requested();
    ui.chord(&[Key::Control, Key::Shift], "Z");
    assert_eq!(ui.pending(), 1);
    assert_eq!(ui.gil().text, "500");
}

#[test]
fn ctrl_z_does_not_match_ctrl_shift_z() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    // Ctrl+Shift+Z is a redo chord: with nothing to redo it changes nothing.
    ui.chord(&[Key::Control, Key::Shift], "Z");
    assert_eq!(ui.pending(), 1, "Ctrl+Shift+Z does not undo");
}

#[test]
fn ctrl_z_with_nothing_to_undo_does_nothing() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.chord(&[Key::Control], "z");
    assert_eq!(ui.pending(), 0);
    assert!(!ui.bridge().get_has_notice());
}

#[test]
fn ctrl_z_in_a_text_box_stays_the_boxs_own() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.navigate(EditorCommand::Party, 0);
    ui.focus_input("Level");
    ui.replace_text("Level", "40");

    ui.chord(&[Key::Control], "z");

    assert_eq!(ui.pending(), 1, "the session history is untouched");
    assert_eq!(ui.gil().text, "500");
}

#[test]
fn the_redo_chord_the_box_knows_stays_with_the_box() {
    let site = editing_site();
    let ui = open_editor(&site);
    undone_with_a_box_focused(&ui);

    box_redo(&ui);

    assert_eq!(ui.pending(), 0);
}

#[test]
fn the_redo_chord_the_box_ignores_reaches_the_page() {
    let site = editing_site();
    let ui = open_editor(&site);
    undone_with_a_box_focused(&ui);

    page_redo(&ui);

    assert_eq!(ui.pending(), 1);
}

#[test]
fn a_dialog_swallows_the_shortcuts() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.ask_discard();

    ui.chord(&[Key::Control], "z");

    assert_eq!(ui.pending(), 1);
    assert!(ui.bridge().get_discard_dialog_open());
}

#[test]
fn the_shortcuts_work_after_a_click_on_the_bar() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.element("Undo", Button).invoke_accessible_default_action();
    assert_eq!(ui.pending(), 0);

    ui.chord(&[Key::Control], "y");

    assert_eq!(ui.pending(), 1, "the page took the focus back");
}
