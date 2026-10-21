//! The modal dialogs and the keyboard: Escape cancels, Enter never confirms a
//! discard, and nothing typed reaches the page behind. Key events go through
//! `Window::dispatch_event` on Slint's testing backend. No window shows and no
//! event loop runs.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Ui, editing_site, open_editor};
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, SharedString};

fn press(ui: &Ui, text: impl Into<SharedString>) {
    let text = text.into();
    ui.window.window().dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    ui.window.window().dispatch_event(WindowEvent::KeyReleased { text });
}

fn key(ui: &Ui, key: Key) {
    press(ui, SharedString::from(key));
}

/// A save with one gil edit, and the discard dialog open over it.
fn discard_dialog() -> (Ui, common::Site) {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.gil().key, "500");
    assert_eq!(ui.pending(), 1);
    ui.window.invoke_back_requested();
    assert!(ui.bridge().get_discard_dialog_open());
    (ui, site)
}

#[test]
fn escape_cancels_the_expert_warning() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.bridge();
    bridge.invoke_expert_toggle_requested();
    assert!(bridge.get_expert_warning_open());

    key(&ui, Key::Escape);

    assert!(!bridge.get_expert_warning_open());
    assert!(!bridge.get_expert_mode());
    assert!(!ui.app.editor.expert());
}

#[test]
fn escape_cancels_the_discard_dialog_and_keeps_the_save() {
    let (ui, _site) = discard_dialog();

    key(&ui, Key::Escape);

    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(ui.session_open());
    assert!(ui.window.get_editor_open());
    assert_eq!(ui.pending(), 1);
    assert_eq!(ui.gil().text, "500");
}

#[test]
fn enter_does_not_confirm_a_discard() {
    let (ui, _site) = discard_dialog();

    key(&ui, Key::Return);

    // The focus starts on Cancel, so Enter cancels.
    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(ui.session_open());
    assert!(ui.window.get_editor_open());
    assert_eq!(ui.pending(), 1);
    assert_eq!(ui.gil().text, "500");
}

#[test]
fn space_on_the_first_button_does_not_confirm_either() {
    let (ui, _site) = discard_dialog();

    press(&ui, " ");

    assert!(!ui.bridge().get_discard_dialog_open(), "Space presses Cancel");
    assert!(ui.session_open());
    assert_eq!(ui.pending(), 1);
}

#[test]
fn tab_reaches_the_discard_button_and_enter_confirms() {
    let (ui, _site) = discard_dialog();

    key(&ui, Key::Tab);
    key(&ui, Key::Return);

    assert!(!ui.session_open(), "the focused Discard button confirms");
    assert!(!ui.window.get_editor_open());
    assert!(!ui.bridge().get_discard_dialog_open());
}

#[test]
fn a_key_behind_a_dialog_reaches_nothing() {
    let (ui, _site) = discard_dialog();
    let gil = ui.gil();

    for digit in ["7", "8", "9"] {
        press(&ui, digit);
    }

    assert!(ui.bridge().get_discard_dialog_open(), "a digit does not close the dialog");
    assert_eq!(ui.pending(), 1);
    assert_eq!(ui.gil().text, gil.text);
    assert!(!ui.bridge().get_has_notice());
}

#[test]
fn escape_still_cancels_after_a_click_on_the_overlay() {
    let (ui, _site) = discard_dialog();
    ui.window.window().set_size(slint::PhysicalSize::new(800, 600));
    let position = LogicalPosition::new(4.0, 4.0);
    ui.window.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    ui.window.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
    assert!(ui.bridge().get_discard_dialog_open(), "the overlay swallows the click");

    key(&ui, Key::Escape);

    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(ui.session_open());
}
