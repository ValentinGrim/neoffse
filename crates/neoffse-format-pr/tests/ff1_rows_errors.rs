//! The refusals of the raw `apply` for `Insert` and `Remove`. A failed row
//! edit changes nothing.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{Row, SlotData, model, model_with_escaped_row, standard_model};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, RowId, Value};
use neoffse_format_pr::ff1::{Ff1Model, names};

const ENFORCE: CapPolicy = CapPolicy::Enforce;
const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;

fn item(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

fn key_item(key: u32) -> RowId {
    RowId::new(FieldId::new(names::KEY_ITEMS), key)
}

fn insert(row: &RowId, position: Option<usize>, values: Vec<(&'static str, Value)>) -> Edit {
    Edit::insert(row.clone(), position, values)
}

fn with_row(row: Row) -> (Ff1Model, String) {
    let mut data = SlotData::standard();
    data.inventory.push(row);
    let text = data.render();
    (model(&text), text)
}

#[test]
fn collections_and_paths() {
    let mut model = standard_model();
    let character = RowId::new(FieldId::new(names::CHARACTER), 9);
    for policy in [ENFORCE, STORAGE_ONLY] {
        assert!(matches!(
            model.apply(&insert(&character, None, Vec::new()), policy),
            Err(EditError::ReadOnly { .. })
        ));
        assert!(matches!(model.apply(&Edit::remove(character.clone()), policy), Err(EditError::ReadOnly { .. })));
        let real = RowId::new(FieldId::new(names::CHARACTER), 1);
        assert!(matches!(model.apply(&Edit::remove(real), policy), Err(EditError::ReadOnly { .. })));
    }
    for path in ["gil", "nothing"] {
        let row = RowId::new(FieldId::new(path), 1);
        let want = FieldId::new(path);
        for edit in [insert(&row, None, Vec::new()), Edit::remove(row.clone())] {
            let result = model.apply(&edit, ENFORCE);
            assert!(matches!(result, Err(EditError::NotACollection { field }) if field == want), "{path}");
        }
    }
}

#[test]
fn unknown_fields_in_values() {
    let mut model = standard_model();
    let name = vec![("name", Value::Text("x".to_owned()))];
    let repeated = vec![(names::COUNT, Value::Int(1)), (names::COUNT, Value::Int(2))];
    for values in [name, repeated] {
        let result = model.apply(&insert(&item(7), None, values), ENFORCE);
        assert!(matches!(result, Err(EditError::UnknownField { .. })));
    }
}

#[test]
fn count_bounds_and_caps() {
    let count = |value| vec![(names::COUNT, Value::Int(value))];
    for policy in [ENFORCE, STORAGE_ONLY] {
        let result = standard_model().apply(&insert(&item(7), None, count(0)), policy);
        assert!(matches!(result, Err(EditError::OutOfRange { min: 1, .. })));
    }
    let result = standard_model().apply(&insert(&item(7), None, count(100)), ENFORCE);
    assert!(matches!(result, Err(EditError::AboveCap { cap: 99, .. })));
    assert!(standard_model().apply(&insert(&item(7), None, count(100)), STORAGE_ONLY).is_ok());
    let result = standard_model().apply(&insert(&item(7), None, count(1)), ENFORCE);
    assert!(result.is_ok());
}

#[test]
fn duplicate_rows() {
    for policy in [ENFORCE, STORAGE_ONLY] {
        let result = standard_model().apply(&insert(&item(89), None, Vec::new()), policy);
        assert!(matches!(result, Err(EditError::DuplicateRow { row }) if row == *item(89).id()));
    }
}

#[test]
fn positions() {
    let mut model = standard_model();
    let len = model.rows(&FieldId::new(names::INVENTORY)).unwrap().len();
    model.apply(&insert(&item(7), Some(len), Vec::new()), ENFORCE).unwrap();
    let text = model.document().to_json();
    let result = model.apply(&insert(&item(8), Some(len + 2), Vec::new()), ENFORCE);
    assert!(
        matches!(result, Err(EditError::PositionOutOfRange { position, len: found, collection }) if position == len + 2 && found == len + 1 && collection == FieldId::new(names::INVENTORY))
    );
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn unknown_ids_under_enforce() {
    let result = standard_model().apply(&insert(&item(999), None, Vec::new()), ENFORCE);
    assert!(matches!(result, Err(EditError::UnknownId { id: 999, .. })));
    assert!(standard_model().apply(&insert(&item(999), None, Vec::new()), STORAGE_ONLY).is_ok());
    let mut other = standard_model();
    let cases = [
        insert(&item(47), None, Vec::new()),
        insert(&key_item(104), None, Vec::new()),
        insert(&key_item(999), None, Vec::new()),
    ];
    for edit in cases {
        assert!(matches!(other.apply(&edit, ENFORCE), Err(EditError::UnknownId { .. })), "{edit:?}");
    }
    assert!(other.apply(&insert(&key_item(47), None, Vec::new()), ENFORCE).is_ok());
    assert!(other.apply(&insert(&item(104), None, Vec::new()), ENFORCE).is_ok());
}

#[test]
fn missing_rows() {
    let mut model = standard_model();
    for row in [item(999), key_item(89)] {
        let result = model.apply(&Edit::remove(row.clone()), ENFORCE);
        assert!(matches!(result, Err(EditError::MissingRow { row: found }) if found == *row.id()));
    }
}

#[test]
fn remove_refuses_a_row_it_could_not_restore() {
    let rows = [
        r#"{"contentId":4,"count":1,"extra":0}"#,
        r#"{"count":1,"contentId":4}"#,
        r#"{"contentId":4,"count":7.0}"#,
        r#"{"contentId": 4,"count":1}"#,
    ];
    for text in rows {
        for policy in [ENFORCE, STORAGE_ONLY] {
            let (mut model, original) = with_row(Row::raw(text));
            let result = model.apply(&Edit::remove(item(4)), policy);
            assert!(matches!(result, Err(EditError::ReadOnly { field }) if field == *item(4).id()), "{text}");
            assert_eq!(model.document().to_json(), original, "{text}");
        }
    }
    let (mut model, _) = with_row(Row::new(4, 1));
    assert!(model.apply(&Edit::remove(item(4)), ENFORCE).is_ok());
}

#[test]
fn remove_refuses_a_row_stored_with_other_escapes() {
    for policy in [ENFORCE, STORAGE_ONLY] {
        let (mut model, original) = model_with_escaped_row(7, 4);
        let result = model.apply(&Edit::remove(item(4)), policy);
        assert!(matches!(result, Err(EditError::ReadOnly { field }) if field == *item(4).id()));
        assert_eq!(model.document().to_json(), original);
    }
    let (mut model, original) = model_with_escaped_row(7, 4);
    let result = model.apply_game(&Edit::remove(item(4)));
    assert!(matches!(result, Err(EditError::ReadOnly { field }) if field == *item(4).id()));
    assert_eq!(model.document().to_json(), original);
}

#[test]
fn a_canonical_row_next_to_an_escaped_one_restores_byte_for_byte() {
    for policy in [ENFORCE, STORAGE_ONLY] {
        let (mut model, original) = model_with_escaped_row(7, 4);
        let inverse = model.apply(&Edit::remove(item(7)), policy).unwrap();
        assert_ne!(model.document().to_json(), original);
        model.apply(&inverse, policy).unwrap();
        assert_eq!(model.document().to_json(), original);
    }
}

#[test]
fn remove_refuses_a_row_whose_inverse_would_not_apply() {
    for policy in [ENFORCE, STORAGE_ONLY] {
        let (mut model, original) = with_row(Row::new(4, 0));
        let result = model.apply(&Edit::remove(item(4)), policy);
        assert!(matches!(result, Err(EditError::OutOfRange { min: 1, .. })));
        assert_eq!(model.document().to_json(), original);
    }
}

#[test]
fn the_error_leaves_the_text_untouched() {
    let mut data = SlotData::standard();
    data.spaced = true;
    data.inventory.push(Row::raw(r#"{"count":1,"contentId":4}"#));
    let text = data.render();
    let mut model = model(&text);
    let edits = [
        (insert(&item(89), None, Vec::new()), ENFORCE),
        (insert(&item(999), None, Vec::new()), ENFORCE),
        (insert(&item(7), Some(99), Vec::new()), ENFORCE),
        (insert(&item(7), None, vec![(names::COUNT, Value::Int(0))]), STORAGE_ONLY),
        (insert(&key_item(89), None, Vec::new()), ENFORCE),
        (Edit::remove(item(999)), ENFORCE),
        (Edit::remove(item(4)), ENFORCE),
        (Edit::remove(RowId::new(FieldId::new(names::CHARACTER), 1)), ENFORCE),
    ];
    for (edit, policy) in edits {
        assert!(model.apply(&edit, policy).is_err(), "{edit:?}");
        assert_eq!(model.document().to_json(), text, "{edit:?}");
    }
}
