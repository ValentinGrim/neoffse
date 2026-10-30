//! The editor menu's navigation state: where it starts, when it resets and
//! what leaves it alone. Bridge only.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{Ui, editing_site, open_editor};
use neoffse_app::EditorCommand;

#[test]
fn a_new_editor_starts_on_the_party_overview() {
    let site = editing_site();
    let ui = open_editor(&site);
    assert_eq!(ui.bridge().get_command(), EditorCommand::Party);
    assert_eq!(ui.bridge().get_character(), -1);
}

#[test]
fn navigation_is_reset_when_another_save_opens() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.navigate(EditorCommand::Magic, 2);
    ui.window.invoke_home_requested();
    assert!(!ui.window.get_editor_open());

    let key = ui.first_slots()[0].key;
    ui.window.invoke_slot_clicked(key);
    ui.manual.run_all();

    assert!(ui.window.get_editor_open());
    assert_eq!(ui.bridge().get_command(), EditorCommand::Party);
    assert_eq!(ui.bridge().get_character(), -1);
}

#[test]
fn an_edit_does_not_move_the_navigation() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.navigate(EditorCommand::Items, 1);

    ui.commit(ui.gil().key, "500");

    assert_eq!(ui.pending(), 1);
    assert_eq!(ui.bridge().get_command(), EditorCommand::Items);
    assert_eq!(ui.bridge().get_character(), 1);
}

#[test]
fn an_open_that_fails_keeps_the_navigation() {
    let site = editing_site();
    let garbage = site.root().join("garbage.bin");
    fs::write(&garbage, b"not a save").unwrap();
    let ui = Ui::scanned(&site, Some(garbage));
    let key = ui.first_slots()[0].key;
    ui.window.invoke_slot_clicked(key);
    ui.manual.run_all();
    assert!(ui.window.get_editor_open());
    ui.navigate(EditorCommand::Magic, 2);
    ui.window.invoke_home_requested();
    assert!(!ui.window.get_editor_open());

    ui.window.invoke_open_file_requested();
    ui.manual.run_all();

    assert!(!ui.window.get_editor_open());
    assert!(ui.window.get_has_open_problem(), "the open failed");
    assert_eq!(ui.bridge().get_command(), EditorCommand::Magic);
    assert_eq!(ui.bridge().get_character(), 2);
}

#[test]
fn the_bridge_still_carries_every_part_d_flag() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.bridge();
    assert!(!bridge.get_blocked());

    bridge.invoke_expert_toggle_requested();
    assert!(bridge.get_expert_warning_open());
    assert!(bridge.get_blocked());
    bridge.invoke_expert_warning_cancelled();
    assert!(!bridge.get_blocked());

    ui.commit(ui.gil().key, "500");
    ui.window.invoke_back_requested();
    assert!(bridge.get_discard_dialog_open());
    assert!(bridge.get_blocked());
    assert!(!bridge.get_expert_mode());
    assert!(!bridge.get_has_notice());
}
