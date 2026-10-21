//! `CollectionMeta`: what a collection accepts and how it checks an `Insert`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_core::{CapPolicy, CollectionMeta, EditError, FieldId, FieldMeta, RowField, RowId, Shape, TableKind, Value};

fn inventory() -> FieldId {
    FieldId::new("inventory")
}

fn row(key: u32) -> RowId {
    RowId::new(inventory(), key)
}

fn key_meta() -> FieldMeta {
    let mut key = FieldMeta::new("Item", Shape::Int { min: 1, max: 200 });
    key.cap = Some(100);
    key.tables.push(TableKind::Item);
    key
}

fn count_field() -> RowField {
    let mut meta = FieldMeta::new("Count", Shape::Int { min: 1, max: 99 });
    meta.cap = Some(50);
    RowField::new("count", meta, Value::Int(1))
}

fn collection() -> CollectionMeta {
    let mut meta = CollectionMeta::new(inventory(), "Inventory", key_meta());
    meta.fields.push(count_field());
    meta
}

fn count(value: Value) -> Vec<(&'static str, Value)> {
    vec![("count", value)]
}

#[test]
fn new_is_insertable_removable_and_empty() {
    let meta = CollectionMeta::new(inventory(), "Inventory", key_meta());
    assert!(meta.insertable && meta.removable);
    assert_eq!(meta.fields, vec![]);
    assert_eq!(meta.id, inventory());
    assert_eq!(meta.label, "Inventory");
    assert_eq!(meta.key, key_meta());
}

#[test]
fn check_insert_accepts_a_valid_row() {
    let meta = collection();
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        meta.check_insert(&row(7), &[], policy).unwrap();
        meta.check_insert(&row(7), &count(Value::Int(4)), policy).unwrap();
        meta.check_insert(&row(7), &count(Value::Int(50)), policy).unwrap();
    }
}

#[test]
fn a_collection_that_is_not_insertable_refuses_insert() {
    let mut meta = collection();
    meta.insertable = false;
    let error = meta.check_insert(&row(7), &[], CapPolicy::StorageOnly).unwrap_err();
    assert!(matches!(error, EditError::ReadOnly { field } if field == inventory()));
    meta.check_remove(&row(7)).unwrap();
}

#[test]
fn a_collection_that_is_not_removable_refuses_remove() {
    let mut meta = collection();
    meta.removable = false;
    let error = meta.check_remove(&row(7)).unwrap_err();
    assert!(matches!(error, EditError::ReadOnly { field } if field == inventory()));
    meta.check_insert(&row(7), &[], CapPolicy::Enforce).unwrap();
}

#[test]
fn a_row_of_another_collection_is_refused() {
    let bag = FieldId::new("bag");
    let stranger = RowId::new(bag.clone(), 7);
    let meta = collection();
    let error = meta.check_insert(&stranger, &[], CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::NotACollection { field } if field == bag));
    let error = meta.check_remove(&stranger).unwrap_err();
    assert!(matches!(error, EditError::NotACollection { field } if field == bag));

    let mut locked = collection();
    locked.insertable = false;
    locked.removable = false;
    let error = locked.check_insert(&stranger, &[], CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::NotACollection { .. }));
    let error = locked.check_remove(&stranger).unwrap_err();
    assert!(matches!(error, EditError::NotACollection { .. }));
}

#[test]
fn the_key_is_checked_against_its_metadata() {
    let meta = collection();
    for key in [0, 201] {
        let error = meta.check_insert(&row(key), &[], CapPolicy::StorageOnly).unwrap_err();
        let EditError::OutOfRange { field, min, max, value } = error else {
            panic!("expected OutOfRange, got {error:?}");
        };
        assert_eq!((field, min, max, value), (row(key).id().clone(), 1, 200, i64::from(key)));
    }
    let error = meta.check_insert(&row(150), &[], CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::AboveCap { value: 150, cap: 100, .. }));
    meta.check_insert(&row(150), &[], CapPolicy::StorageOnly).unwrap();
}

#[test]
fn the_key_is_checked_before_the_values() {
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        let error = collection().check_insert(&row(0), &count(Value::Int(100)), policy).unwrap_err();
        assert!(
            matches!(
                error,
                EditError::OutOfRange { ref field, value: 0, min: 1, max: 200 } if *field == *row(0).id()
            ),
            "{error:?}"
        );
    }
}

#[test]
fn an_unknown_value_name_is_refused() {
    let error = collection()
        .check_insert(&row(7), &[("weight", Value::Int(1))], CapPolicy::Enforce)
        .unwrap_err();
    assert!(matches!(error, EditError::UnknownField { field } if field == row(7).field("weight")));
}

#[test]
fn a_repeated_value_name_is_refused() {
    let values = vec![("count", Value::Int(2)), ("count", Value::Int(3))];
    let error = collection().check_insert(&row(7), &values, CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::UnknownField { field } if field == row(7).field("count")));
}

#[test]
fn values_are_checked_against_the_row_template() {
    let meta = collection();
    let error = meta.check_insert(&row(7), &count(Value::Int(100)), CapPolicy::StorageOnly).unwrap_err();
    assert!(matches!(
        error,
        EditError::OutOfRange {
            value: 100,
            min: 1,
            max: 99,
            ..
        }
    ));
    let error = meta.check_insert(&row(7), &count(Value::Text("x".into())), CapPolicy::StorageOnly).unwrap_err();
    assert!(matches!(error, EditError::WrongType { .. }));
    let error = meta.check_insert(&row(7), &count(Value::Int(60)), CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::AboveCap { value: 60, cap: 50, .. }));
    meta.check_insert(&row(7), &count(Value::Int(60)), CapPolicy::StorageOnly).unwrap();
}

#[test]
fn check_remove_accepts_any_key() {
    collection().check_remove(&row(0)).unwrap();
}

fn assert_send_sync_static<T: Send + Sync + 'static>() {}

#[test]
fn collection_types_are_send_sync_and_static() {
    assert_send_sync_static::<CollectionMeta>();
    assert_send_sync_static::<RowField>();
}

#[test]
fn row_field_new_keeps_its_arguments() {
    let field = count_field();
    assert_eq!(field.name, "count");
    assert_eq!(field.initial, Value::Int(1));
    assert_eq!(field.meta.cap, Some(50));
}
