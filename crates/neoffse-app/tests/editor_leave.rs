//! The leave question: the view model alone, then the bridge.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::sync::Arc;

use common::{Editing, Part, editing, editing_site, field, open_editor};
use neoffse_app::editor::{Input, Leave, LeaveAnswer};

fn gil_key(ed: &Editing) -> i32 {
    field(&ed.data(), Part::Inventory, "Gil", "Gil").key
}

/// Makes the session dirty with one gil edit.
fn dirty(ed: &Editing) {
    assert!(ed.editor().commit(gil_key(ed), Input::Text("500".to_owned())).is_none());
    assert!(ed.editor().is_dirty());
}

fn gil_text(ed: &Editing) -> String {
    field(&ed.data(), Part::Inventory, "Gil", "Gil").text.to_string()
}

#[test]
fn a_clean_session_leaves_at_once() {
    let site = editing_site();
    let ed = editing(&site);
    assert_eq!(ed.editor().leave(Leave::Back), LeaveAnswer::Left);
    assert!(!ed.flow.state.is_open());
    assert!(!ed.editor().leave_pending());
}

#[test]
fn leaving_with_no_save_is_a_clean_leave() {
    let site = editing_site();
    let ed = editing(&site);
    ed.editor().back();
    assert_eq!(ed.editor().leave(Leave::Close), LeaveAnswer::Left);
    assert!(!ed.editor().leave_pending());
}

#[test]
fn a_dirty_session_asks() {
    let site = editing_site();
    let ed = editing(&site);
    dirty(&ed);
    assert_eq!(ed.editor().leave(Leave::Back), LeaveAnswer::Ask);
    assert!(ed.flow.state.is_open());
    assert!(ed.editor().leave_pending());
}

#[test]
fn an_edit_undone_to_the_clean_point_is_not_dirty() {
    let site = editing_site();
    let ed = editing(&site);
    dirty(&ed);
    ed.undo();
    assert_eq!(ed.editor().leave(Leave::Back), LeaveAnswer::Left);
}

#[test]
fn cancel_keeps_the_session_and_the_edits() {
    let site = editing_site();
    let ed = editing(&site);
    dirty(&ed);
    assert_eq!(ed.editor().leave(Leave::Back), LeaveAnswer::Ask);
    ed.editor().cancel_leave();

    assert!(!ed.editor().leave_pending());
    assert!(ed.flow.state.is_open());
    assert_eq!(gil_text(&ed), "500");
}

#[test]
fn discard_closes_the_session() {
    let site = editing_site();
    let ed = editing(&site);
    dirty(&ed);
    let weak = Arc::downgrade(&ed.session());
    assert_eq!(ed.editor().leave(Leave::Back), LeaveAnswer::Ask);

    assert_eq!(ed.editor().confirm_discard(), Some(Leave::Back));
    assert!(!ed.flow.state.is_open());
    assert!(weak.upgrade().is_none(), "the session is dropped");
    assert!(!ed.editor().leave_pending());
}

#[test]
fn confirm_without_a_request_does_nothing() {
    let site = editing_site();
    let ed = editing(&site);
    dirty(&ed);
    assert_eq!(ed.editor().confirm_discard(), None);
    assert!(ed.flow.state.is_open());
    assert_eq!(gil_text(&ed), "500");
}

#[test]
fn a_second_request_replaces_the_first() {
    let site = editing_site();
    let ed = editing(&site);
    dirty(&ed);
    assert_eq!(ed.editor().leave(Leave::Back), LeaveAnswer::Ask);
    assert_eq!(ed.editor().leave(Leave::Close), LeaveAnswer::Ask);
    assert_eq!(ed.editor().confirm_discard(), Some(Leave::Close));
}

#[test]
fn back_on_a_dirty_save_opens_the_dialog() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.bridge();
    ui.commit(ui.gil().key, "500");

    ui.window.invoke_back_requested();
    assert!(bridge.get_discard_dialog_open());
    assert!(ui.window.get_editor_open());
    assert!(ui.session_open());

    bridge.invoke_discard_cancelled();
    assert!(!bridge.get_discard_dialog_open());
    assert!(ui.window.get_editor_open());
    assert!(ui.session_open());
    assert_eq!(ui.gil().text, "500");

    ui.window.invoke_back_requested();
    bridge.invoke_discard_confirmed();
    assert!(!ui.window.get_editor_open());
    assert!(!bridge.get_discard_dialog_open());
    assert!(!ui.session_open());
}

#[test]
fn back_on_a_clean_save_goes_home_at_once() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.window.invoke_back_requested();
    assert!(!ui.window.get_editor_open());
    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(!ui.session_open());
}

#[test]
fn a_discard_with_nothing_pending_is_harmless() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.gil().key, "500");
    ui.bridge().invoke_discard_confirmed();

    assert!(ui.window.get_editor_open());
    assert!(ui.session_open());
    assert!(!ui.bridge().get_discard_dialog_open());
    assert_eq!(ui.gil().text, "500");
}
