//! `Insert`, `Remove` and their inverses on an ordered collection.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Model, insert, int, remove, row};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, Value};

const ENFORCE: CapPolicy = CapPolicy::Enforce;

fn inventory() -> FieldId {
    FieldId::new("inventory")
}

fn count_of(model: &Model, key: u32) -> Option<i64> {
    model.snapshot().rows.into_iter().find(|(k, _)| *k == key).map(|(_, count)| count)
}

#[test]
fn insert_appends_by_default() {
    let mut model = Model::sample();
    let inverse = model.apply(&insert(5, None, Some(4)), ENFORCE).unwrap();
    assert_eq!(model.rows(), [3, 7, 9, 5]);
    assert_eq!(inverse, Edit::Remove { row: row(5) });
}

#[test]
fn insert_at_a_position() {
    for (position, expected) in [(0, [5, 3, 7, 9]), (2, [3, 7, 5, 9]), (3, [3, 7, 9, 5])] {
        let mut model = Model::sample();
        model.apply(&insert(5, Some(position), None), ENFORCE).unwrap();
        assert_eq!(model.rows(), expected, "position {position}");
    }
}

#[test]
fn insert_past_the_end_is_refused() {
    let mut model = Model::sample();
    let before = model.snapshot();
    let error = model.apply(&insert(5, Some(4), None), ENFORCE).unwrap_err();
    assert!(matches!(
        error,
        EditError::PositionOutOfRange { ref collection, position: 4, len: 3 } if *collection == inventory()
    ));
    assert_eq!(model.snapshot(), before);
}

#[test]
fn insert_a_duplicate_key_is_refused() {
    let mut model = Model::sample();
    let before = model.snapshot();
    let error = model.apply(&insert(7, None, Some(50)), ENFORCE).unwrap_err();
    assert!(matches!(error, EditError::DuplicateRow { ref row } if row.to_string() == "inventory.7"));
    assert_eq!(model.snapshot(), before);
}

#[test]
fn insert_a_key_outside_the_table_is_refused_under_enforce() {
    let mut model = Model::sample();
    let error = model.apply(&insert(150, None, None), ENFORCE).unwrap_err();
    assert!(matches!(
        error,
        EditError::UnknownId { ref field, id: 150 } if *field == *row(150).id()
    ));
    assert_eq!(model.rows(), [3, 7, 9]);
}

#[test]
fn insert_a_key_outside_the_table_passes_under_storage_only() {
    let mut model = Model::sample();
    let inverse = model.apply(&insert(150, None, None), CapPolicy::StorageOnly).unwrap();
    assert_eq!(model.rows(), [3, 7, 9, 150]);
    assert_eq!(inverse, Edit::Remove { row: row(150) });
}

#[test]
fn insert_a_key_outside_the_storage_bound_is_out_of_range() {
    for (key, policy) in [(0, ENFORCE), (201, ENFORCE), (0, CapPolicy::StorageOnly), (201, CapPolicy::StorageOnly)] {
        let mut model = Model::sample();
        let error = model.apply(&insert(key, None, None), policy).unwrap_err();
        assert!(
            matches!(
                error,
                EditError::OutOfRange { ref field, value, min: 1, max: 200 } if *field == *row(key).id() && value == i64::from(key)
            ),
            "key {key}: {error:?}"
        );
        assert_eq!(model.rows(), [3, 7, 9]);
    }
}

#[test]
fn insert_into_something_that_is_no_collection() {
    let gil = FieldId::new("gil");
    let character = FieldId::new("character").with_id(1);
    for path in [gil, character] {
        let mut model = Model::sample();
        let before = model.snapshot();
        let target = neoffse_core::RowId::new(path.clone(), 5);
        for edit in [Edit::insert(target.clone(), None, Vec::new()), Edit::remove(target)] {
            let error = model.apply(&edit, ENFORCE).unwrap_err();
            assert!(matches!(error, EditError::NotACollection { ref field } if *field == path), "{path}");
        }
        assert_eq!(model.snapshot(), before);
    }
}

#[test]
fn insert_fills_defaults_and_takes_given_values() {
    let mut model = Model::sample();
    model.apply(&insert(5, None, None), ENFORCE).unwrap();
    assert_eq!(count_of(&model, 5), Some(1));
    model.apply(&insert(6, None, Some(4)), ENFORCE).unwrap();
    assert_eq!(count_of(&model, 6), Some(4));

    let before = model.snapshot();
    let bad = |values: Vec<(&'static str, Value)>| Edit::insert(row(8), None, values);
    let mut refused = Vec::new();
    for edit in [
        insert(8, None, Some(100)),
        bad(vec![("weight", Value::Int(1))]),
        bad(vec![("count", Value::Int(1)), ("count", Value::Int(2))]),
        bad(vec![("count", Value::Text("x".into()))]),
    ] {
        refused.push(model.apply(&edit, ENFORCE).unwrap_err());
    }
    assert!(matches!(
        refused[0],
        EditError::OutOfRange {
            value: 100,
            min: 1,
            max: 99,
            ..
        }
    ));
    assert!(matches!(refused[1], EditError::UnknownField { ref field } if field.to_string() == "inventory.8.weight"));
    assert!(matches!(refused[2], EditError::UnknownField { .. }));
    assert!(matches!(refused[3], EditError::WrongType { .. }));
    assert_eq!(model.snapshot(), before);
}

#[test]
fn remove_returns_an_insert_at_the_old_position() {
    let mut model = Model::sample();
    let inverse = model.apply(&remove(7), ENFORCE).unwrap();
    assert_eq!(
        inverse,
        Edit::Insert {
            row: row(7),
            position: Some(1),
            values: vec![("count", Value::Int(2))]
        }
    );
    assert_eq!(model.rows(), [3, 9]);
}

#[test]
fn remove_then_the_inverse_restores_the_order() {
    for key in [3, 7, 9] {
        let mut model = Model::sample();
        let before = model.snapshot();
        let inverse = model.apply(&remove(key), ENFORCE).unwrap();
        model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
        assert_eq!(model.snapshot(), before, "key {key}");
        assert_eq!(model.rows(), [3, 7, 9]);
    }
}

#[test]
fn remove_a_missing_row_is_refused() {
    let mut model = Model::sample();
    let error = model.apply(&remove(4), ENFORCE).unwrap_err();
    assert!(matches!(error, EditError::MissingRow { ref row } if row.to_string() == "inventory.4"));
    assert_eq!(model.rows(), [3, 7, 9]);
}

#[test]
fn insert_then_remove_round_trips() {
    let mut model = Model::sample();
    let before = model.snapshot();
    let inverse = model.apply(&insert(5, Some(1), Some(4)), ENFORCE).unwrap();
    let again = model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
    assert_eq!(model.snapshot(), before);
    assert_eq!(
        again,
        Edit::Insert {
            row: row(5),
            position: Some(1),
            values: vec![("count", Value::Int(4))]
        }
    );
}

#[test]
fn redo_of_an_append_differs_only_in_position() {
    let original = insert(5, None, Some(4));
    let mut model = Model::sample();
    let start = model.clone();
    let remove_back = model.apply(&original, ENFORCE).unwrap();
    let redo = model.apply(&remove_back, CapPolicy::StorageOnly).unwrap();
    assert_eq!(
        redo,
        Edit::Insert {
            row: row(5),
            position: Some(3),
            values: vec![("count", Value::Int(4))]
        }
    );
    assert_ne!(redo, original);

    let mut with_original = start.clone();
    with_original.apply(&original, CapPolicy::StorageOnly).unwrap();
    let mut with_redo = start;
    with_redo.apply(&redo, CapPolicy::StorageOnly).unwrap();
    assert_eq!(with_original.snapshot(), with_redo.snapshot());
}

#[test]
fn a_set_through_an_absent_row_is_refused() {
    let mut model = Model::sample();
    let error = model.apply(&int(row(4).field("count"), 9), ENFORCE).unwrap_err();
    assert!(matches!(error, EditError::MissingRow { ref row } if row.to_string() == "inventory.4"));

    let inverse = model.apply(&int(row(7).field("count"), 9), ENFORCE).unwrap();
    assert_eq!(count_of(&model, 7), Some(9));
    model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
    assert_eq!(count_of(&model, 7), Some(2));
}

#[test]
fn a_changed_count_survives_remove_and_undo() {
    let mut model = Model::sample();
    model.apply(&int(row(7).field("count"), 9), ENFORCE).unwrap();
    let inverse = model.apply(&remove(7), ENFORCE).unwrap();
    model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
    assert_eq!(model.rows(), [3, 7, 9]);
    assert_eq!(count_of(&model, 7), Some(9));
}

#[test]
fn target_names_the_field_or_the_row() {
    let field = FieldId::new("gil");
    assert_eq!(int(field.clone(), 1).target(), &field);
    assert_eq!(insert(5, None, None).target(), row(5).id());
    assert_eq!(remove(5).target(), row(5).id());
}

#[test]
fn constructors_build_the_variants() {
    let field = FieldId::new("gil");
    assert_eq!(Edit::set(field.clone(), Value::Int(1)), Edit::Set { field, value: Value::Int(1) });
    let values = vec![("count", Value::Int(2))];
    assert_eq!(
        Edit::insert(row(5), Some(1), values.clone()),
        Edit::Insert {
            row: row(5),
            position: Some(1),
            values
        }
    );
    assert_eq!(Edit::remove(row(5)), Edit::Remove { row: row(5) });
}
