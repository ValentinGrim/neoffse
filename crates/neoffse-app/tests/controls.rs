//! The custom controls, driven through window events on Slint's testing
//! backend. No window shows and no event loop runs. This file covers buttons,
//! focus order and accessibility; `controls_input.rs` covers edits and checks.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod controls_harness;

use controls_harness::{Theme, ThemeId, by_label, click, click_at, harness, key, press};
use i_slint_backend_testing::AccessibleRole;
use slint::ComponentHandle;
use slint::platform::{Key, WindowEvent};

#[test]
fn a_button_activates_on_enter_and_space() {
    let ui = harness();
    ui.invoke_focus_first();
    assert!(ui.get_first_focus());
    key(&ui, Key::Return);
    assert_eq!(ui.get_clicks(), 1);
    press(&ui, " ");
    assert_eq!(ui.get_clicks(), 2);
    press(&ui, "a");
    key(&ui, Key::Escape);
    assert_eq!(ui.get_clicks(), 2);
}

#[test]
fn a_click_activates_once_on_release() {
    let ui = harness();
    click(&ui, 20.0, 20.0);
    assert_eq!(ui.get_clicks(), 1);
    click_at(&ui, (20.0, 20.0), (300.0, 300.0));
    assert_eq!(ui.get_clicks(), 1, "a release outside is no click");
}

#[test]
fn a_click_gives_the_button_the_focus() {
    let ui = harness();
    click(&ui, 20.0, 20.0);
    assert!(ui.get_first_focus());
}

#[test]
fn a_disabled_button_ignores_keys_and_pointer() {
    let ui = harness();
    ui.invoke_focus_off();
    assert!(!ui.get_off_focus());
    key(&ui, Key::Return);
    press(&ui, " ");
    click(&ui, 20.0, 60.0);
    assert_eq!(ui.get_off_clicks(), 0);
    assert!(!ui.get_off_focus());
}

#[test]
fn tab_moves_focus_in_source_order_and_skips_disabled_controls() {
    let ui = harness();
    ui.invoke_focus_first();
    key(&ui, Key::Tab);
    assert!(ui.get_edit_focus(), "the disabled button is skipped");
    key(&ui, Key::Tab);
    assert!(ui.get_check_focus());
    ui.window().dispatch_event(WindowEvent::KeyPressed { text: Key::Shift.into() });
    key(&ui, Key::Tab);
    assert!(ui.get_edit_focus());
    key(&ui, Key::Tab);
    assert!(ui.get_first_focus());
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: Key::Shift.into() });
}

#[test]
fn the_controls_set_their_accessibility_properties() {
    let ui = harness();
    let button = by_label(&ui, "First");
    assert_eq!(button.accessible_role(), Some(AccessibleRole::Button));
    assert_eq!(button.accessible_enabled(), Some(true));
    assert_eq!(by_label(&ui, "Off").accessible_enabled(), Some(false));

    let edit = by_label(&ui, "Name");
    assert_eq!(edit.accessible_role(), Some(AccessibleRole::TextInput));
    ui.invoke_focus_edit();
    press(&ui, "7");
    assert_eq!(edit.accessible_value().as_deref(), Some("7"));

    let check = by_label(&ui, "Option");
    assert_eq!(check.accessible_role(), Some(AccessibleRole::Checkbox));
    assert_eq!(check.accessible_checked(), Some(false));
    ui.set_checked(true);
    assert_eq!(check.accessible_checked(), Some(true));

    let icon = by_label(&ui, "Close it");
    assert_eq!(icon.accessible_role(), Some(AccessibleRole::Button));
}

#[test]
fn the_default_accessible_action_activates() {
    let ui = harness();
    by_label(&ui, "First").invoke_accessible_default_action();
    by_label(&ui, "Close it").invoke_accessible_default_action();
    by_label(&ui, "Option").invoke_accessible_default_action();
    assert_eq!((ui.get_clicks(), ui.get_icon_clicks()), (1, 1));
    assert!(ui.get_checked());
}

#[test]
fn a_shell_look_and_a_theme_look_both_build() {
    let ui = harness();
    click(&ui, 20.0, 260.0);
    click(&ui, 20.0, 300.0);
    assert_eq!((ui.get_shell_clicks(), ui.get_theme_clicks()), (1, 1));
    ui.global::<Theme>().set_id(ThemeId::Pr);
    click(&ui, 20.0, 300.0);
    assert_eq!(ui.get_theme_clicks(), 2);
}
