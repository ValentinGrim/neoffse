//! The leave reason `Restore`: remembered without closing anything, and kept
//! open on Discard until the restore has replaced the file.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Editing, Part, editing, editing_site, field};
use neoffse_app::editor::{Input, Leave, LeaveAnswer};

fn dirty(ed: &Editing) {
    let key = field(&ed.data(), Part::Inventory, "Gil", "Gil").key;
    assert!(ed.editor().commit(key, Input::Text("500".to_owned())).is_none());
    assert!(ed.editor().is_dirty());
}

#[test]
fn request_discard_remembers_and_closes_nothing() {
    let site = editing_site();
    let ed = editing(&site);
    dirty(&ed);
    ed.editor().request_discard(Leave::Restore);
    assert!(ed.editor().leave_pending());
    assert!(ed.flow.state.is_open());
    assert!(ed.editor().is_dirty());
}

#[test]
fn request_discard_on_a_clean_session_closes_nothing_either() {
    let site = editing_site();
    let ed = editing(&site);
    ed.editor().request_discard(Leave::Restore);
    assert!(ed.editor().leave_pending());
    assert!(ed.flow.state.is_open());
}

#[test]
fn confirm_discard_for_a_restore_keeps_the_session() {
    let site = editing_site();
    let ed = editing(&site);
    dirty(&ed);
    ed.editor().request_discard(Leave::Restore);
    assert_eq!(ed.editor().confirm_discard(), Some(Leave::Restore));
    assert!(ed.flow.state.is_open());
    assert!(ed.editor().is_dirty());
    assert!(!ed.editor().leave_pending());
}

#[test]
fn confirm_discard_for_back_still_closes_the_session() {
    let site = editing_site();
    let ed = editing(&site);
    dirty(&ed);
    assert_eq!(ed.editor().leave(Leave::Back), LeaveAnswer::Ask);
    assert_eq!(ed.editor().confirm_discard(), Some(Leave::Back));
    assert!(!ed.flow.state.is_open());
}

#[test]
fn cancel_leave_forgets_a_restore() {
    let site = editing_site();
    let ed = editing(&site);
    dirty(&ed);
    ed.editor().request_discard(Leave::Restore);
    ed.editor().cancel_leave();
    assert!(!ed.editor().leave_pending());
    assert_eq!(ed.editor().confirm_discard(), None);
    assert!(ed.flow.state.is_open());
}

#[test]
fn a_close_request_replaces_a_pending_restore() {
    let site = editing_site();
    let ed = editing(&site);
    dirty(&ed);
    ed.editor().request_discard(Leave::Restore);
    assert_eq!(ed.editor().leave(Leave::Close), LeaveAnswer::Ask);
    assert_eq!(ed.editor().confirm_discard(), Some(Leave::Close));
    assert!(!ed.flow.state.is_open());
}
