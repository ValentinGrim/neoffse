//! The refusals of `apply_game`. Each one leaves the text untouched and
//! passes on the raw `apply`, which holds no game rule.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{Row, SlotData, character_field, model, standard_model};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, RowId, Value};
use neoffse_format_pr::ff1::{Ff1Model, names};

const ENFORCE: CapPolicy = CapPolicy::Enforce;
const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;
const STORAGE_MAX: i64 = 2_147_483_647;

fn slot(id: u32, number: u32) -> FieldId {
    character_field(id, names::EQUIPMENT).with_id(number)
}

fn count(key: u32) -> FieldId {
    FieldId::new(names::INVENTORY).with_id(key).with_name(names::COUNT)
}

fn row(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

fn key_count(key: u32) -> FieldId {
    FieldId::new(names::KEY_ITEMS).with_id(key).with_name(names::COUNT)
}

fn key_row(key: u32) -> RowId {
    RowId::new(FieldId::new(names::KEY_ITEMS), key)
}

fn set(field: FieldId, value: i64) -> Edit {
    Edit::set(field, Value::Int(value))
}

/// `apply_game` of `edit` on `model`: the error, with the text unchanged.
fn refused(model: &mut Ff1Model, edit: &Edit) -> EditError {
    let text = model.document().to_json();
    let error = model.apply_game(edit).unwrap_err();
    assert_eq!(model.document().to_json(), text, "{}", edit.target());
    error
}

fn raw_accepts(model: &Ff1Model, edit: &Edit, policy: CapPolicy) {
    let mut copy = model.clone();
    assert!(copy.apply(edit, policy).is_ok(), "raw {}", edit.target());
}

fn assert_read_only(model: &mut Ff1Model, edit: &Edit, field: &FieldId) {
    match refused(model, edit) {
        EditError::ReadOnly { field: got } => assert_eq!(&got, field),
        other => panic!("{}: {other}", edit.target()),
    }
}

#[test]
fn a_slot_count_cannot_be_set_directly() {
    let mut model = standard_model();
    let field = slot(1, 1).with_name(names::COUNT);
    let edit = set(field.clone(), 3);
    assert_read_only(&mut model, &edit, &field);
    raw_accepts(&model, &edit, ENFORCE);
}

#[test]
fn a_key_item_count_cannot_be_set_directly() {
    let mut model = standard_model();
    let field = key_count(45);
    for value in [1, 2, 99, 100, 0, -1, STORAGE_MAX + 1] {
        assert_read_only(&mut model, &set(field.clone(), value), &field);
    }
    // The raw interface keeps the storage bounds and no game cap.
    let mut raw = model.clone();
    assert!(raw.apply(&set(field.clone(), 100), ENFORCE).is_ok());
    assert!(raw.apply(&set(field.clone(), STORAGE_MAX), ENFORCE).is_ok());
    for value in [0, -1, STORAGE_MAX + 1] {
        let result = model.clone().apply(&set(field.clone(), value), ENFORCE);
        assert!(matches!(result, Err(EditError::OutOfRange { min: 1, max: STORAGE_MAX, .. })), "{value}");
    }
}

#[test]
fn a_key_item_insert_holds_one() {
    let mut model = standard_model();
    for value in [2, 0, -1, 100] {
        let insert = Edit::insert(key_row(47), None, vec![(names::COUNT, Value::Int(value))]);
        match refused(&mut model, &insert) {
            EditError::OutOfRange { field, value: got, min, max } => assert_eq!((field, got, min, max), (key_count(47), value, 1, 1)),
            other => panic!("{other}"),
        }
        if value > 0 {
            raw_accepts(&model, &insert, ENFORCE);
        }
    }
    for values in [vec![(names::COUNT, Value::Int(1))], Vec::new()] {
        let mut copy = model.clone();
        assert!(copy.apply_game(&Edit::insert(key_row(47), None, values)).is_ok());
        assert_eq!(copy.get(&key_count(47)).unwrap(), Value::Int(1));
    }
    // A wrong kind keeps its own error.
    let text = Edit::insert(key_row(47), None, vec![(names::COUNT, Value::Text("x".to_owned()))]);
    assert!(matches!(refused(&mut model, &text), EditError::WrongType { .. }));
}

#[test]
fn a_key_item_can_still_be_removed() {
    let mut model = standard_model();
    assert!(model.apply_game(&Edit::remove(key_row(45))).is_ok());
}

#[test]
fn a_count_below_the_wearers_is_refused() {
    let mut model = standard_model();
    let low = set(count(183), 1);
    match refused(&mut model, &low) {
        EditError::OutOfRange { field, value, min, max } => assert_eq!((field, value, min, max), (count(183), 1, 2, STORAGE_MAX)),
        other => panic!("{other}"),
    }
    raw_accepts(&model, &low, ENFORCE);
    assert!(model.apply_game(&set(count(183), 2)).is_ok());

    let mut data = SlotData::standard();
    data.characters[0].equipment[0].content_id = "104".to_owned();
    data.characters[3].equipment[0].content_id = "104".to_owned();
    let mut model = model_of(&data);
    for values in [vec![(names::COUNT, Value::Int(1))], Vec::new()] {
        let insert = Edit::insert(row(104), None, values);
        match refused(&mut model, &insert) {
            EditError::OutOfRange { field, min, .. } => assert_eq!((field, min), (count(104), 2)),
            other => panic!("{other}"),
        }
        raw_accepts(&model, &insert, ENFORCE);
    }
    let enough = Edit::insert(row(104), None, vec![(names::COUNT, Value::Int(2))]);
    assert!(model.apply_game(&enough).is_ok());
}

fn model_of(data: &SlotData) -> Ff1Model {
    model(&data.render())
}

#[test]
fn worn_rows_cannot_be_removed() {
    let mut model = standard_model();
    for key in [89, 202] {
        let edit = Edit::remove(row(key));
        assert_read_only(&mut model, &edit, row(key).id());
        raw_accepts(&model, &edit, ENFORCE);
    }
    assert!(model.apply_game(&Edit::remove(row(100))).is_ok());
}

#[test]
fn placeholder_rows_cannot_be_touched() {
    let mut model = standard_model();
    for edit in [set(count(62), 3), Edit::remove(row(62)), set(count(133), 3), Edit::remove(row(130))] {
        assert_read_only(&mut model, &edit, &edit.target().clone());
        raw_accepts(&model, &edit, ENFORCE);
    }
    let mut data = SlotData::standard();
    data.inventory.drain(..5);
    let mut model = model_of(&data);
    let insert = Edit::insert(row(62), None, Vec::new());
    assert_read_only(&mut model, &insert, row(62).id());
    raw_accepts(&model, &insert, ENFORCE);
    let absent = set(count(62), 3);
    assert_read_only(&mut model, &absent, &count(62));
}

#[test]
fn a_row_the_model_cannot_restore_is_refused_on_both_interfaces() {
    let texts = [
        r#"{"contentId":4,"count":1,"extra":0}"#,
        r#"{"count":1,"contentId":4}"#,
        r#"{"contentId":4,"count":7.0}"#,
        r#"{"contentId": 4,"count":1}"#,
    ];
    for text in texts {
        let mut data = SlotData::standard();
        data.inventory.push(Row::raw(text));
        let mut model = model_of(&data);
        let edit = Edit::remove(row(4));
        assert_read_only(&mut model, &edit, row(4).id());
        for policy in [ENFORCE, STORAGE_ONLY] {
            let mut copy = model.clone();
            assert!(matches!(copy.apply(&edit, policy), Err(EditError::ReadOnly { .. })), "{text}");
        }
    }
}

#[test]
fn caps_and_tables_are_always_enforced() {
    let mut model = standard_model();
    let level = set(character_field(1, names::LEVEL), 100);
    assert!(matches!(refused(&mut model, &level), EditError::AboveCap { cap: 99, .. }));
    let job = set(character_field(1, names::JOB), 13);
    assert!(matches!(refused(&mut model, &job), EditError::UnknownId { id: 13, .. }));
    let equip = set(slot(1, 1), 165);
    assert!(matches!(refused(&mut model, &equip), EditError::UnknownId { id: 165, .. }));
    for edit in [level, job, equip] {
        raw_accepts(&model, &edit, STORAGE_ONLY);
    }
}

#[test]
fn equip_ids_follow_the_slot_table() {
    for (number, id) in [(1, 165), (2, 89), (1, 45), (1, 999), (1, 0), (1, 130)] {
        let mut model = standard_model();
        let error = refused(&mut model, &set(slot(1, number), id));
        assert!(
            matches!(error, EditError::UnknownId { id: got, .. } if got == id),
            "slot {number} id {id}: {error}"
        );
    }
    for (number, id) in [(1, 62), (2, 165), (3, 131), (4, 132), (5, 133)] {
        let mut model = standard_model();
        assert!(model.apply_game(&set(slot(1, number), id)).is_ok(), "slot {number} id {id}");
    }
}

#[test]
fn unknown_character_and_slot() {
    let mut model = standard_model();
    match refused(&mut model, &set(slot(9, 1), 100)) {
        EditError::MissingRow { row } => assert_eq!(row, FieldId::new(names::CHARACTER).with_id(9)),
        other => panic!("{other}"),
    }
    for number in [0, 6] {
        let error = refused(&mut model, &set(slot(1, number), 100));
        assert!(matches!(error, EditError::UnknownField { .. }), "slot {number}");
    }
}

#[test]
fn wrong_values_are_refused_before_the_plan() {
    let mut model = standard_model();
    let text = Edit::set(slot(1, 1), Value::Text("x".to_owned()));
    assert!(matches!(refused(&mut model, &text), EditError::WrongType { .. }));
    assert!(matches!(refused(&mut model, &set(slot(1, 1), -1)), EditError::OutOfRange { .. }));
}

#[test]
fn a_refusal_changes_nothing() {
    let mut data = SlotData::standard();
    data.spaced = true;
    data.inventory.push(Row::raw(r#"{"count":1,"contentId":4}"#));
    let text = data.render();
    let mut model = model(&text);
    let edits = [
        set(slot(1, 1).with_name(names::COUNT), 3),
        set(count(183), 1),
        Edit::remove(row(89)),
        set(count(62), 3),
        Edit::remove(row(4)),
        set(character_field(1, names::LEVEL), 100),
        set(slot(1, 1), 165),
        set(slot(9, 1), 100),
        set(slot(1, 6), 100),
        Edit::set(slot(1, 1), Value::Text("x".to_owned())),
    ];
    for edit in edits {
        assert!(model.apply_game(&edit).is_err(), "{}", edit.target());
        assert_eq!(model.document().to_json(), text, "{}", edit.target());
    }
}

#[test]
fn a_locked_field_is_read_only_whatever_the_value() {
    let mut model = standard_model();
    let field = slot(1, 1).with_name(names::COUNT);
    for value in [Value::Text("x".to_owned()), Value::Int(-1), Value::Int(STORAGE_MAX + 1)] {
        assert_read_only(&mut model, &Edit::set(field.clone(), value), &field);
    }
}
