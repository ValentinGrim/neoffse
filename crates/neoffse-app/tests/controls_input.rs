//! The custom controls, driven through window events on Slint's testing
//! backend. No window shows and no event loop runs. This file covers edits and
//! checks; `controls.rs` covers buttons, focus order and accessibility.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod controls_harness;

use controls_harness::{click, harness, key, press};
use slint::platform::Key;

#[test]
fn an_edit_takes_typing_and_enter_accepts() {
    let ui = harness();
    ui.invoke_focus_edit();
    press(&ui, "1");
    press(&ui, "2");
    assert_eq!(ui.get_typed(), "12");
    assert_eq!(ui.get_accepts(), 0);
    key(&ui, Key::Return);
    assert_eq!(ui.get_typed(), "12");
    assert_eq!(ui.get_accepts(), 1);
}

#[test]
fn escape_reaches_the_owner_of_the_edit() {
    let ui = harness();
    ui.invoke_focus_edit();
    key(&ui, Key::Escape);
    assert_eq!(ui.get_escapes(), 1);
    assert_eq!(ui.get_screen_escapes(), 0, "a handled Escape stops at the edit");
}

#[test]
fn an_edit_passes_escape_on_when_its_owner_declines() {
    let ui = harness();
    ui.set_escape_handled(false);
    ui.invoke_focus_edit();
    key(&ui, Key::Escape);
    assert_eq!(ui.get_escapes(), 1);
    assert_eq!(ui.get_screen_escapes(), 1, "the screen behind sees the key");
    assert!(ui.get_edit_focus(), "an unhandled Escape does not drop the focus");
}

#[test]
fn a_disabled_edit_takes_no_focus_and_no_typing() {
    let ui = harness();
    ui.invoke_focus_locked_edit();
    click(&ui, 20.0, 340.0);
    press(&ui, "x");
    key(&ui, Key::Return);
    assert!(!ui.get_locked_edit_focus());
    assert_eq!(ui.get_locked_text(), "fixed");
    assert_eq!(ui.get_accepts(), 0);
}

#[test]
fn a_check_toggles_on_space_and_click() {
    let ui = harness();
    ui.invoke_focus_check();
    press(&ui, " ");
    assert!(ui.get_checked());
    click(&ui, 20.0, 140.0);
    assert!(!ui.get_checked());
    key(&ui, Key::Return);
    assert!(ui.get_checked());
    assert_eq!(ui.get_toggles(), 3);
}

#[test]
fn a_disabled_check_does_not_toggle() {
    let ui = harness();
    ui.invoke_focus_locked();
    press(&ui, " ");
    click(&ui, 20.0, 180.0);
    assert!(!ui.get_locked_checked());
}
