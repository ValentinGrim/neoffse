//! Clicks and Tab on the editor menu. The positions come from the render of a
//! 1280 by 800 window on the testing backend; if the layout moves, move them
//! with it. No window shows and no event loop runs.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Ui, editing_site, open_editor};
use neoffse_app::EditorCommand;
use slint::ComponentHandle;
use slint::platform::Key;

/// A shown-size editor: the layout the click positions below assume.
fn sized_ui() -> (Ui, common::Site) {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.window.window().set_size(slint::PhysicalSize::new(1280, 800));
    (ui, site)
}

#[test]
fn a_click_on_a_card_opens_its_page_and_escape_still_steps_back() {
    let (ui, _site) = sized_ui();

    ui.click_at(700.0, 380.0);
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

    ui.click_at(80.0, 182.0);
    assert_eq!(ui.bridge().get_command(), EditorCommand::Items);
    assert_eq!(ui.bridge().get_character(), -1);

    ui.click_at(80.0, 216.0);
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

    ui.click_at(700.0, 380.0);
    ui.click_at(80.0, 182.0);

    assert_eq!(ui.bridge().get_command(), EditorCommand::Party);
    assert_eq!(ui.bridge().get_character(), -1);
}
