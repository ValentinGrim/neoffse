//! The game interface of `SaveDocument` (`apply_game`, `row_locked`,
//! `field_locked`) and `GameChange`, over the fake format, which has no game
//! rules: the provided bodies apply here.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::fake::{add, doc, gil, hp, inventory, row};
use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FieldId, GameChange, SaveDocument, Value};

fn set(field: FieldId, value: i64) -> Edit {
    Edit::set(field, Value::Int(value))
}

fn value_of(document: &mut dyn SaveDocument, field: &FieldId) -> Value {
    document.view().unwrap().find(field).unwrap().1.value.clone()
}

#[test]
fn apply_game_defaults_to_one_enforced_edit() {
    let mut document = doc();
    let edit = set(hp(1), 70);
    let change = document.apply_game(&edit).unwrap();
    assert_eq!(change.forward.edits(), [edit]);
    assert_eq!(change.inverse.edits(), [set(hp(1), 50)]);
    assert_eq!(value_of(&mut *document, &hp(1)), Value::Int(70));
    document.apply_all(&change.inverse, CapPolicy::StorageOnly).unwrap();
    assert_eq!(value_of(&mut *document, &hp(1)), Value::Int(50));
    document.apply_all(&change.forward, CapPolicy::StorageOnly).unwrap();
    assert_eq!(value_of(&mut *document, &hp(1)), Value::Int(70));
}

#[test]
fn apply_game_covers_the_row_edits() {
    let mut document = doc();
    let insert = add(5, None, 7);
    let change = document.apply_game(&insert).unwrap();
    assert_eq!(change.forward.edits(), [insert]);
    assert_eq!(change.inverse.edits(), [Edit::remove(row(5))]);
    document.apply_all(&change.inverse, CapPolicy::StorageOnly).unwrap();
    assert!(document.view().unwrap().find(&row(5).field("count")).is_none());
}

#[test]
fn the_default_enforces_the_cap() {
    let mut document = doc();
    let over = set(hp(1), 1_000);
    let error = document.apply_game(&over).unwrap_err();
    assert!(matches!(error, EditError::AboveCap { .. }), "{error:?}");
    assert_eq!(value_of(&mut *document, &hp(1)), Value::Int(50), "a refused edit changes nothing");
    document.apply(&over, CapPolicy::StorageOnly).unwrap();
    assert_eq!(value_of(&mut *document, &hp(1)), Value::Int(1_000));
}

#[test]
fn the_default_passes_other_refusals_through() {
    let mut document = doc();
    let error = document.apply_game(&set(FieldId::new("nothing"), 1)).unwrap_err();
    assert!(matches!(error, EditError::UnknownField { .. }), "{error:?}");
    let error = document.apply_game(&set(gil(), -1)).unwrap_err();
    assert!(matches!(error, EditError::OutOfRange { .. }), "{error:?}");
}

#[test]
fn the_default_locks_nothing() {
    let mut document = doc();
    for key in [2, 3, 99] {
        assert!(!document.row_locked(&row(key)).unwrap(), "inventory.{key}");
    }
    let unknown_row = neoffse_core::RowId::new(FieldId::new("nothing"), 1);
    assert!(!document.row_locked(&unknown_row).unwrap());
    for field in [gil(), hp(1), row(2).field("count"), FieldId::new("nothing"), inventory()] {
        assert!(!document.field_locked(&field).unwrap(), "{field}");
    }
}

#[test]
fn the_methods_dispatch_through_a_boxed_document() {
    let mut document: Box<dyn SaveDocument> = doc();
    assert!(!document.row_locked(&row(2)).unwrap());
    assert!(!document.field_locked(&gil()).unwrap());
    let change = document.apply_game(&set(gil(), 5)).unwrap();
    assert_eq!(change.forward.len(), 1);
    assert_eq!(change.inverse.len(), 1);
}

#[test]
fn game_change_new_keeps_both_sets() {
    let forward: ChangeSet = [set(gil(), 1), set(gil(), 2)].into_iter().collect();
    let inverse: ChangeSet = [set(gil(), 0)].into_iter().collect();
    let change = GameChange::new(forward.clone(), inverse.clone());
    assert_eq!(change.forward, forward);
    assert_eq!(change.inverse, inverse);
    assert_eq!(change.clone(), change);
    assert_ne!(change, GameChange::new(inverse, forward));
}
