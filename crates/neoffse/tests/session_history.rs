//! `Session`: the edit history, the dirty count and discard.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::open_session;
use neoffse::Session;
use neoffse::core::Edit;
use neoffse::core::{Capability, EditError, FieldId, Value};
use neoffse::pr::ff1::names;

fn gil() -> FieldId {
    FieldId::new(names::GIL)
}

fn level() -> FieldId {
    FieldId::new(names::CHARACTER).with_id(1).with_name(names::LEVEL)
}

fn set(field: &FieldId, value: i64) -> Edit {
    Edit::set(field.clone(), Value::Int(value))
}

fn value(session: &mut Session, field: &FieldId) -> Value {
    let view = session.view().unwrap();
    let (_, field) = view.find(field).expect("the field is in the view");
    field.value.clone()
}

fn gil_value(session: &mut Session) -> i64 {
    let Value::Int(gil) = value(session, &gil()) else {
        panic!("gil is an integer")
    };
    gil
}

#[test]
fn a_new_session_is_clean() {
    let (_dir, mut session) = open_session();
    assert_eq!(session.pending_count(), 0);
    assert!(!session.is_dirty());
    assert!(!session.can_undo());
    assert!(!session.can_redo());
    assert!(!session.undo().unwrap());
    assert!(!session.redo().unwrap());
}

#[test]
fn edit_undo_redo_moves_the_value_and_the_dirty_state() {
    let (_dir, mut session) = open_session();
    let before = gil_value(&mut session);
    session.edit(&set(&gil(), before + 1)).unwrap();
    assert_eq!(gil_value(&mut session), before + 1);
    assert_eq!(session.pending_count(), 1);
    assert!(session.is_dirty());
    assert!(session.can_undo());

    assert!(session.undo().unwrap());
    assert_eq!(gil_value(&mut session), before);
    assert!(!session.is_dirty());
    assert!(session.can_redo());

    assert!(session.redo().unwrap());
    assert_eq!(gil_value(&mut session), before + 1);
    assert_eq!(session.pending_count(), 1);
    assert!(!session.can_redo());
}

#[test]
fn undoing_every_edit_restores_the_view() {
    let (_dir, mut session) = open_session();
    let view = session.view().unwrap();
    let before = gil_value(&mut session);
    session.edit(&set(&gil(), before + 1)).unwrap();
    session.edit(&set(&gil(), before + 2)).unwrap();
    session.edit(&set(&level(), 2)).unwrap();
    assert_eq!(session.pending_count(), 3);
    for _ in 0..3 {
        assert!(session.undo().unwrap());
    }
    assert!(!session.is_dirty());
    assert_eq!(session.view().unwrap(), view);
}

#[test]
fn a_fork_below_the_clean_point_counts_one() {
    let (_dir, mut session) = open_session();
    let base = gil_value(&mut session);
    session.edit(&set(&gil(), base + 1)).unwrap(); // A
    session.edit(&set(&gil(), base + 2)).unwrap(); // B
    session.undo().unwrap();
    session.undo().unwrap();
    session.edit(&set(&gil(), base + 3)).unwrap(); // C
    assert!(session.is_dirty());
    assert_eq!(session.pending_count(), 1);
    assert!(!session.can_redo());

    // A second fork from the clean point.
    session.undo().unwrap();
    session.edit(&set(&gil(), base + 1)).unwrap(); // A
    session.undo().unwrap();
    session.edit(&set(&gil(), base + 2)).unwrap(); // B
    assert_eq!(session.pending_count(), 1);
}

#[test]
fn undoing_a_forked_edit_returns_to_clean() {
    let (_dir, mut session) = open_session();
    let base = gil_value(&mut session);
    session.edit(&set(&gil(), base + 1)).unwrap();
    session.edit(&set(&gil(), base + 2)).unwrap();
    session.undo().unwrap();
    session.undo().unwrap();
    session.edit(&set(&gil(), base + 3)).unwrap();
    session.undo().unwrap();
    assert_eq!(session.pending_count(), 0);
    assert!(!session.is_dirty());
    assert_eq!(gil_value(&mut session), base);
}

#[test]
fn a_new_edit_clears_the_redo_stack() {
    let (_dir, mut session) = open_session();
    let base = gil_value(&mut session);
    session.edit(&set(&gil(), base + 1)).unwrap();
    session.undo().unwrap();
    assert!(session.can_redo());
    session.edit(&set(&gil(), base + 2)).unwrap();
    assert!(!session.can_redo());
    assert!(!session.redo().unwrap());
}

#[test]
fn a_failed_edit_changes_nothing() {
    let (_dir, mut session) = open_session();
    let base = gil_value(&mut session);
    session.edit(&set(&gil(), base + 1)).unwrap();
    session.undo().unwrap();
    session.edit(&set(&gil(), base + 5)).unwrap();
    session.undo().unwrap();
    session.edit(&set(&gil(), base + 6)).unwrap();
    let level_before = value(&mut session, &level());

    let unknown = set(&FieldId::new("nonsense"), 1);
    assert!(matches!(session.edit(&unknown), Err(EditError::UnknownField { .. })));
    assert!(matches!(session.edit(&set(&level(), 100)), Err(EditError::AboveCap { .. })));
    assert_eq!(session.pending_count(), 1);
    assert!(session.can_undo());
    assert!(!session.can_redo());
    assert_eq!(value(&mut session, &level()), level_before);
    assert_eq!(gil_value(&mut session), base + 6);
}

#[test]
fn a_failed_edit_keeps_the_redo_stack() {
    let (_dir, mut session) = open_session();
    let base = gil_value(&mut session);
    session.edit(&set(&gil(), base + 1)).unwrap();
    session.undo().unwrap();
    assert!(session.edit(&set(&level(), 100)).is_err());
    assert!(session.can_redo());
}

#[test]
fn expert_mode_accepts_a_level_above_the_cap_and_undoes_it() {
    let (_dir, mut session) = open_session();
    let before = value(&mut session, &level());
    assert!(matches!(session.edit(&set(&level(), 100)), Err(EditError::AboveCap { .. })));
    session.edit_expert(&set(&level(), 100)).unwrap();
    assert_eq!(value(&mut session, &level()), Value::Int(100));
    assert_eq!(session.pending_count(), 1);
    assert!(session.undo().unwrap());
    assert_eq!(value(&mut session, &level()), before);
    assert!(session.redo().unwrap());
    assert_eq!(value(&mut session, &level()), Value::Int(100));
}

#[test]
fn discard_restores_the_original() {
    let (_dir, mut session) = open_session();
    let view = session.view().unwrap();
    let base = gil_value(&mut session);
    session.edit(&set(&gil(), base + 1)).unwrap();
    session.edit_expert(&set(&level(), 100)).unwrap();
    session.undo().unwrap();
    session.discard().unwrap();
    assert_eq!(gil_value(&mut session), base);
    assert_eq!(session.view().unwrap(), view);
    assert!(!session.is_dirty());
    assert_eq!(session.pending_count(), 0);
    assert!(!session.can_undo());
    assert!(!session.can_redo());
}

#[test]
fn the_reads_forward_to_the_document() {
    let (_dir, mut session) = open_session();
    assert!(session.capabilities().contains(&Capability::Party));
    assert!(session.summary().is_ok());
    assert!(session.field_locked(&gil()).is_ok());
}
