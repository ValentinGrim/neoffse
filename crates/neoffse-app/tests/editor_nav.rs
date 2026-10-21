//! Back, Home and Escape on the editor page. Back steps up one level: a
//! character page or another command returns to the Party cards, and the
//! cards leave like Home. Key events go through `Window::dispatch_event` on
//! Slint's testing backend; no window shows and no event loop runs.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Ui, editing_site, open_editor};
use neoffse_app::EditorCommand;
use slint::platform::Key;

#[test]
fn back_on_a_character_page_returns_to_the_cards() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.navigate(EditorCommand::Party, 2);

    ui.window.invoke_back_requested();

    assert!(ui.on_cards());
    assert!(ui.window.get_editor_open());
    assert!(ui.session_open());
    assert!(!ui.bridge().get_discard_dialog_open(), "stepping back asks nothing");
    assert_eq!(ui.pending(), 1);
}

#[test]
fn back_on_the_top_level_leaves_a_clean_save() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.window.invoke_back_requested();
    assert!(!ui.window.get_editor_open());
    assert!(!ui.session_open());
}

#[test]
fn back_on_the_top_level_asks_about_unsaved_edits() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.window.invoke_back_requested();
    assert!(ui.bridge().get_discard_dialog_open());
    assert!(ui.window.get_editor_open());
    assert!(ui.session_open());
}

fn back_from_a_command_then_leave(command: EditorCommand) {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.navigate(command, -1);

    ui.window.invoke_back_requested();
    assert!(ui.on_cards());
    assert!(ui.window.get_editor_open());
    assert!(ui.session_open());

    ui.window.invoke_back_requested();
    assert!(!ui.window.get_editor_open());
    assert!(!ui.session_open());
}

#[test]
fn back_on_items_returns_to_the_cards_then_leaves() {
    back_from_a_command_then_leave(EditorCommand::Items);
}

#[test]
fn back_on_magic_returns_to_the_cards_then_leaves() {
    back_from_a_command_then_leave(EditorCommand::Magic);
}

#[test]
fn back_on_items_with_unsaved_edits_asks_only_from_the_cards() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.navigate(EditorCommand::Items, -1);

    ui.window.invoke_back_requested();
    assert!(ui.on_cards());
    assert!(!ui.bridge().get_discard_dialog_open());

    ui.window.invoke_back_requested();
    assert!(ui.bridge().get_discard_dialog_open());
    assert!(ui.window.get_editor_open());
}

#[test]
fn a_stale_character_index_on_another_command_still_steps_back() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.navigate(EditorCommand::Magic, 3);

    ui.window.invoke_back_requested();

    assert!(ui.on_cards());
    assert!(ui.window.get_editor_open());
}

#[test]
fn home_from_a_character_page_leaves_a_clean_save_at_once() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.navigate(EditorCommand::Party, 2);
    ui.window.invoke_home_requested();
    assert!(!ui.window.get_editor_open());
    assert!(!ui.session_open());
}

#[test]
fn home_from_a_character_page_asks_about_unsaved_edits() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.navigate(EditorCommand::Party, 2);
    ui.window.invoke_home_requested();
    assert!(ui.bridge().get_discard_dialog_open());
    assert!(ui.window.get_editor_open());
    assert_eq!(ui.bridge().get_character(), 2, "the page stays until the user decides");
}

/// The part D sequence: dirty leave, cancel, leave again, confirm.
fn leave_flow(leave: impl Fn(&Ui)) {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();

    leave(&ui);
    assert!(ui.bridge().get_discard_dialog_open());
    ui.bridge().invoke_discard_cancelled();
    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(ui.session_open());
    assert_eq!(ui.gil().text, "500");

    leave(&ui);
    ui.bridge().invoke_discard_confirmed();
    assert!(!ui.window.get_editor_open());
    assert!(!ui.session_open());
}

#[test]
fn the_existing_leave_flow_is_unchanged_through_back() {
    leave_flow(|ui| ui.window.invoke_back_requested());
}

#[test]
fn the_existing_leave_flow_is_unchanged_through_home() {
    leave_flow(|ui| ui.window.invoke_home_requested());
}

#[test]
fn escape_steps_back_like_the_back_button() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.navigate(EditorCommand::Party, 2);

    ui.key(Key::Escape);
    assert!(ui.on_cards());
    assert!(ui.window.get_editor_open());

    ui.key(Key::Escape);
    assert!(!ui.window.get_editor_open());
    assert!(!ui.session_open());
}

#[test]
fn escape_on_items_returns_to_the_cards() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.navigate(EditorCommand::Items, -1);

    ui.key(Key::Escape);

    assert!(ui.on_cards());
    assert!(ui.window.get_editor_open());
}

#[test]
fn escape_with_a_dialog_open_cancels_the_dialog_not_the_page() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.window.invoke_back_requested();
    assert!(ui.bridge().get_discard_dialog_open());

    ui.key(Key::Escape);

    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(ui.window.get_editor_open());
    assert!(ui.session_open());
}

#[test]
fn escape_on_home_does_nothing() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.window.invoke_home_requested();
    assert!(!ui.window.get_editor_open());
    ui.navigate(EditorCommand::Magic, 1);

    ui.key(Key::Escape);

    assert!(!ui.window.get_editor_open());
    assert!(!ui.bridge().get_discard_dialog_open());
    assert_eq!(ui.bridge().get_command(), EditorCommand::Magic);
    assert_eq!(ui.bridge().get_character(), 1);
}

#[test]
fn the_page_takes_the_focus_back_after_the_expert_warning_closes() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.navigate(EditorCommand::Party, 2);
    ui.bridge().invoke_expert_toggle_requested();
    assert!(ui.bridge().get_expert_warning_open());
    ui.key(Key::Escape);
    assert!(!ui.bridge().get_expert_warning_open());
    assert_eq!(ui.bridge().get_character(), 2, "Escape closed the dialog only");

    ui.key(Key::Escape);

    assert!(ui.on_cards(), "the page handles Escape again");
}

#[test]
fn the_page_takes_the_focus_back_after_the_discard_dialog_closes() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.window.invoke_back_requested();
    ui.bridge().invoke_discard_cancelled();
    assert!(!ui.bridge().get_discard_dialog_open());

    ui.key(Key::Escape);

    assert!(ui.bridge().get_discard_dialog_open(), "Escape on the cards asks again");
}
