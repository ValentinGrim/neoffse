//! `row_locked`: which rows the game interface will refuse to remove,
//! whatever the values. It reads the document and writes nothing.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{Row, SlotData, character_field, model, model_with_escaped_row, standard_model};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, RowId, Value};
use neoffse_format_pr::ff1::names;

fn item(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

fn key_item(key: u32) -> RowId {
    RowId::new(FieldId::new(names::KEY_ITEMS), key)
}

#[test]
fn placeholder_rows_are_locked() {
    let mut model = standard_model();
    for key in [62, 130, 131, 132, 133] {
        assert!(model.row_locked(&item(key)).unwrap(), "inventory.{key}");
    }
}

#[test]
fn worn_rows_are_locked() {
    let mut model = standard_model();
    for key in [89, 183, 202] {
        assert!(model.row_locked(&item(key)).unwrap(), "inventory.{key}");
    }
    for key in [17, 100, 103, 165] {
        assert!(!model.row_locked(&item(key)).unwrap(), "inventory.{key}");
    }
    assert!(!model.row_locked(&key_item(45)).unwrap());
}

#[test]
fn character_rows_are_locked() {
    let mut model = standard_model();
    assert!(model.row_locked(&RowId::new(FieldId::new(names::CHARACTER), 1)).unwrap());
}

#[test]
fn a_non_canonical_row_is_locked() {
    let rows = [
        r#"{"contentId":4,"count":1,"extra":0}"#,
        r#"{"count":1,"contentId":4}"#,
        r#"{"contentId":4,"count":7.0}"#,
        r#"{"contentId": 4,"count":1}"#,
    ];
    for text in rows {
        let mut data = SlotData::standard();
        data.inventory.push(Row::raw(text));
        let mut model = model(&data.render());
        assert!(model.row_locked(&item(4)).unwrap(), "{text}");
    }
    let mut data = SlotData::standard();
    data.inventory.push(Row::new(4, 1));
    assert!(!model(&data.render()).row_locked(&item(4)).unwrap());
}

#[test]
fn a_row_stored_with_other_escapes_is_locked() {
    let (mut model, _) = model_with_escaped_row(7, 4);
    assert!(model.row_locked(&item(4)).unwrap());
    assert!(!model.row_locked(&item(7)).unwrap());
}

#[test]
fn a_zero_count_is_canonical_and_stays_unlocked() {
    let mut data = SlotData::standard();
    data.inventory.push(Row::new(4, 0));
    assert!(!model(&data.render()).row_locked(&item(4)).unwrap());
}

#[test]
fn the_answer_follows_the_document() {
    let mut model = standard_model();
    let slot = character_field(1, names::EQUIPMENT).with_id(1);
    model.apply(&Edit::set(slot.clone(), Value::Int(62)), CapPolicy::Enforce).unwrap();
    assert!(!model.row_locked(&item(89)).unwrap());
    assert!(!model.row_locked(&item(100)).unwrap());
    model.apply(&Edit::set(slot, Value::Int(100)), CapPolicy::Enforce).unwrap();
    assert!(model.row_locked(&item(100)).unwrap());
}

#[test]
fn a_removed_row_leaves_the_others_as_they_were() {
    let mut model = standard_model();
    model.apply(&Edit::remove(item(100)), CapPolicy::Enforce).unwrap();
    assert!(model.row_locked(&item(89)).unwrap());
    assert!(!model.row_locked(&item(103)).unwrap());
}

#[test]
fn absent_and_foreign_rows() {
    let mut model = standard_model();
    let result = model.row_locked(&item(999));
    assert!(matches!(result, Err(EditError::MissingRow { row }) if row == *item(999).id()));
    let result = model.row_locked(&key_item(89));
    assert!(matches!(result, Err(EditError::MissingRow { .. })));
    let result = model.row_locked(&RowId::new(FieldId::new(names::CHARACTER), 9));
    assert!(matches!(result, Err(EditError::MissingRow { .. })));
    for path in ["gil", "nothing"] {
        let result = model.row_locked(&RowId::new(FieldId::new(path), 1));
        assert!(
            matches!(result, Err(EditError::NotACollection { field }) if field == FieldId::new(path)),
            "{path}"
        );
    }
}

#[test]
fn row_locked_changes_nothing() {
    let mut data = SlotData::standard();
    data.spaced = true;
    data.inventory.push(Row::raw(r#"{"count":1,"contentId":4}"#));
    let text = data.render();
    let mut model = model(&text);
    let rows = [
        item(62),
        item(89),
        item(100),
        item(4),
        item(999),
        key_item(45),
        RowId::new(FieldId::new("gil"), 1),
    ];
    for row in rows {
        let _ = model.row_locked(&row);
        assert_eq!(model.document().to_json(), text, "{}", row.id());
    }
}
