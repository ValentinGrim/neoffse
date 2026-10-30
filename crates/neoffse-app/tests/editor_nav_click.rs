//! Clicks and Tab on the editor menu. Each click lands on the centre of the
//! element it targets, read from the accessibility tree of the testing
//! backend, so the tests follow the layout instead of pinning pixels. No
//! window shows and no event loop runs.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Ui, editing_site, open_editor};
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use neoffse_app::EditorCommand;
use slint::ComponentHandle;
use slint::platform::Key;

/// An editor at a fixed size, so the layout is the same on every run.
fn sized_ui() -> (Ui, common::Site) {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.window.window().set_size(slint::PhysicalSize::new(1280, 800));
    (ui, site)
}

/// The second party card, Bram's.
fn second_card(ui: &Ui) -> ElementHandle {
    ElementHandle::find_by_element_type_name(&ui.window, "CharacterRow")
        .nth(1)
        .expect("a second card")
}

#[test]
fn a_click_on_a_card_opens_its_page_and_escape_still_steps_back() {
    let (ui, _site) = sized_ui();

    ui.click_element(&second_card(&ui));
    assert_eq!(ui.bridge().get_command(), EditorCommand::Party);
    assert_eq!(ui.bridge().get_character(), 1, "the second card is Bram's");

    ui.key(Key::Escape);
    assert!(ui.on_cards(), "the card vanished and the page kept the focus");
    assert!(ui.window.get_editor_open());
}

#[test]
fn a_click_on_a_command_shows_its_page_and_resets_the_character() {
    let (ui, _site) = sized_ui();
    ui.navigate(EditorCommand::Party, 2);

    ui.click_element(&ui.element("Items", AccessibleRole::Button));
    assert_eq!(ui.bridge().get_command(), EditorCommand::Items);
    assert_eq!(ui.bridge().get_character(), -1);

    ui.click_element(&ui.element("Magic", AccessibleRole::Button));
    assert_eq!(ui.bridge().get_command(), EditorCommand::Magic);

    ui.key(Key::Escape);
    assert!(ui.on_cards());
}

#[test]
fn enter_on_a_focused_command_row_selects_it() {
    let (ui, _site) = sized_ui();
    // Tab from the page root visits Home, Back, Party, then Items.
    for _ in 0..4 {
        ui.key(Key::Tab);
    }

    ui.key(Key::Return);

    assert_eq!(ui.bridge().get_command(), EditorCommand::Items);
}

#[test]
fn a_dialog_blocks_clicks_on_cards_and_commands() {
    let (ui, _site) = sized_ui();
    ui.bridge().invoke_expert_toggle_requested();
    assert!(ui.bridge().get_blocked());

    ui.click_element(&second_card(&ui));
    ui.click_element(&ui.element("Items", AccessibleRole::Button));

    assert_eq!(ui.bridge().get_command(), EditorCommand::Party);
    assert_eq!(ui.bridge().get_character(), -1);
}
