//! Expert mode on the bridge: the warning, the flags, the offer after an
//! above-cap refusal. No window shows and no event loop runs.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{editing_site, open_editor};
use neoffse_app::EditProblem;
use neoffse_app::editor::{Leave, LeaveAnswer};

/// Toggles expert mode on through the warning.
fn turn_on(ui: &common::Ui) {
    let bridge = ui.bridge();
    bridge.invoke_expert_toggle_requested();
    bridge.invoke_expert_warning_confirmed();
}

#[test]
fn expert_starts_off_and_is_not_persisted() {
    let site = editing_site();
    let ui = open_editor(&site);
    assert!(!ui.app.editor.expert());
    assert!(!ui.bridge().get_expert_mode());
    assert!(!ui.bridge().get_expert_menu_checked());
    assert!(!ui.bridge().get_expert_warning_open());
}

#[test]
fn turning_on_asks_first() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.bridge();
    bridge.invoke_expert_toggle_requested();

    assert!(bridge.get_expert_warning_open());
    assert!(!bridge.get_expert_mode());
    assert!(!bridge.get_expert_menu_checked(), "the checkmark reverts until the user confirms");
    assert!(!ui.app.editor.expert());
}

#[test]
fn a_checkmark_toggled_by_slint_before_the_callback_is_rewritten() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.bridge();
    // Slint flips `checked` before it calls `activated`.
    bridge.set_expert_menu_checked(true);
    bridge.invoke_expert_toggle_requested();
    assert!(!bridge.get_expert_menu_checked());
}

#[test]
fn confirming_turns_it_on() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.bridge();
    turn_on(&ui);

    assert!(bridge.get_expert_mode());
    assert!(bridge.get_expert_menu_checked());
    assert!(!bridge.get_expert_warning_open());
    assert!(ui.app.editor.expert());

    ui.commit(ui.level().key, "100");
    assert!(!bridge.get_has_notice());
    assert_eq!(ui.level().text, "100");
}

#[test]
fn cancelling_changes_nothing() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.bridge();
    bridge.invoke_expert_toggle_requested();
    bridge.invoke_expert_warning_cancelled();

    assert!(!bridge.get_expert_mode());
    assert!(!bridge.get_expert_menu_checked());
    assert!(!bridge.get_expert_warning_open());
    assert!(!ui.app.editor.expert());
    assert_eq!(ui.pending(), 0);
}

#[test]
fn turning_off_needs_no_warning() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.bridge();
    turn_on(&ui);
    assert!(ui.slot_count().editable);

    bridge.invoke_expert_toggle_requested();
    assert!(!bridge.get_expert_mode());
    assert!(!bridge.get_expert_menu_checked());
    assert!(!bridge.get_expert_warning_open());
    assert!(!ui.slot_count().editable);
}

#[test]
fn above_cap_offers_the_switch() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.bridge();
    let level = ui.level().key;

    ui.commit(level, "100");
    assert_eq!(bridge.get_notice().problem, EditProblem::AboveCap);
    assert!(bridge.get_offer_expert());

    bridge.invoke_notice_dismissed();
    assert!(!bridge.get_offer_expert(), "dismissing the notice clears the offer");

    ui.commit(ui.gil().key, "abc");
    assert!(bridge.get_has_notice());
    assert!(!bridge.get_offer_expert(), "another refusal offers nothing");

    ui.commit(level, "100");
    assert!(bridge.get_offer_expert());
    bridge.invoke_expert_toggle_requested();
    bridge.invoke_expert_warning_cancelled();
    assert!(bridge.get_has_notice(), "cancelling keeps the notice");
    assert!(bridge.get_offer_expert(), "and its offer");
    bridge.invoke_expert_toggle_requested();
    bridge.invoke_expert_warning_confirmed();
    assert!(!bridge.get_has_notice(), "the switch clears the notice");
    assert!(!bridge.get_offer_expert());

    ui.commit(level, "100");
    assert!(!bridge.get_has_notice(), "in expert mode no notice appears");
    assert!(!bridge.get_offer_expert());
}

#[test]
fn the_locks_grey_only_in_the_default_mode() {
    let site = editing_site();
    let ui = open_editor(&site);
    assert!(!ui.slot_count().editable);
    assert!(!ui.spell().editable);

    turn_on(&ui);
    assert!(ui.slot_count().editable);
    assert!(!ui.spell().editable, "a read-only field stays a label");
}

#[test]
fn expert_edits_go_through_the_raw_interface() {
    let site = editing_site();
    let ui = open_editor(&site);
    let key = ui.slot_count().key;
    let before = ui.slot_count().text;

    ui.commit(key, "5");
    assert!(ui.bridge().get_has_notice(), "the default mode refuses a locked field");
    assert_eq!(ui.pending(), 0);

    turn_on(&ui);
    let key = ui.slot_count().key;
    ui.commit(key, "5");
    assert!(!ui.bridge().get_has_notice());
    assert_ne!(before, "5");
    assert_eq!(ui.slot_count().text, "5");
    assert_eq!(ui.pending(), 1);
}

#[test]
fn closing_the_save_turns_expert_off() {
    let site = editing_site();
    let ui = open_editor(&site);
    turn_on(&ui);
    assert!(ui.app.editor.expert());

    assert_eq!(ui.app.editor.leave(Leave::Back), LeaveAnswer::Left);
    assert!(!ui.app.editor.expert());
}

#[test]
fn back_through_the_window_resets_the_bridge_flags() {
    let site = editing_site();
    let ui = open_editor(&site);
    turn_on(&ui);
    ui.window.invoke_back_requested();

    assert!(!ui.window.get_editor_open());
    assert!(!ui.bridge().get_expert_mode());
    assert!(!ui.bridge().get_expert_menu_checked());
}
