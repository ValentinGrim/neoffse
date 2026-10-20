//! `FieldMeta::check` and the helpers around it.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_core::{CapPolicy, EditError, FieldId, FieldMeta, NamedValue, Shape, TableKind, Value, ValueKind};

const BOTH: [CapPolicy; 2] = [CapPolicy::Enforce, CapPolicy::StorageOnly];

fn id() -> FieldId {
    FieldId::new("character").with_id(1).with_name("hp")
}

fn int(min: i64, max: i64) -> FieldMeta {
    FieldMeta::new("HP", Shape::Int { min, max })
}

fn capped() -> FieldMeta {
    let mut meta = int(0, 9_999);
    meta.cap = Some(999);
    meta
}

fn check(meta: &FieldMeta, value: &Value, policy: CapPolicy) -> Result<(), EditError> {
    meta.check(&id(), value, policy)
}

#[test]
fn check_accepts_the_bounds_and_what_lies_between() {
    let meta = int(0, 9_999);
    for policy in BOTH {
        for value in [0, 1, 9_998, 9_999] {
            check(&meta, &Value::Int(value), policy).unwrap();
        }
    }
}

#[test]
fn check_rejects_one_past_each_bound() {
    let meta = int(0, 9_999);
    for policy in BOTH {
        for value in [-1, 10_000] {
            let error = check(&meta, &Value::Int(value), policy).unwrap_err();
            let EditError::OutOfRange { field, value: found, min, max } = error else {
                panic!("expected OutOfRange, got {error:?}");
            };
            assert_eq!((field, found, min, max), (id(), value, 0, 9_999));
        }
    }
}

#[test]
fn check_handles_the_extremes_of_i64() {
    let wide = int(i64::MIN, i64::MAX);
    for value in [i64::MIN, i64::MAX] {
        check(&wide, &Value::Int(value), CapPolicy::Enforce).unwrap();
    }
    let narrow = int(0, 5);
    for value in [i64::MIN, i64::MAX] {
        let error = check(&narrow, &Value::Int(value), CapPolicy::Enforce).unwrap_err();
        assert!(matches!(error, EditError::OutOfRange { .. }));
    }
}

#[test]
fn an_inverted_range_accepts_nothing() {
    let meta = int(5, 1);
    for value in [1, 3, 5] {
        let error = check(&meta, &Value::Int(value), CapPolicy::StorageOnly).unwrap_err();
        assert!(matches!(error, EditError::OutOfRange { min: 5, max: 1, .. }));
    }
}

#[test]
fn check_rejects_a_value_of_the_wrong_kind() {
    let cases = [
        (Shape::Int { min: 0, max: 9 }, Value::Bool(true), ValueKind::Int, ValueKind::Bool),
        (Shape::Int { min: 0, max: 9 }, Value::Text("x".into()), ValueKind::Int, ValueKind::Text),
        (Shape::Bool, Value::Int(1), ValueKind::Bool, ValueKind::Int),
        (Shape::Text, Value::Int(1), ValueKind::Text, ValueKind::Int),
    ];
    for (shape, value, expected, found) in cases {
        let error = check(&FieldMeta::new("x", shape), &value, CapPolicy::Enforce).unwrap_err();
        let EditError::WrongType { expected: e, found: f, .. } = error else {
            panic!("expected WrongType, got {error:?}");
        };
        assert_eq!((e, f), (expected, found));
    }
}

#[test]
fn bool_and_text_shapes_accept_their_own_kind() {
    for policy in BOTH {
        check(&FieldMeta::new("Flag", Shape::Bool), &Value::Bool(false), policy).unwrap();
        check(&FieldMeta::new("Name", Shape::Text), &Value::Text(String::new()), policy).unwrap();
    }
}

#[test]
fn read_only_comes_first() {
    let mut meta = int(0, 9);
    meta.read_only = true;
    for policy in BOTH {
        for value in [Value::Int(3), Value::Bool(true), Value::Int(99)] {
            let error = check(&meta, &value, policy).unwrap_err();
            assert!(matches!(error, EditError::ReadOnly { field } if field == id()));
        }
    }
}

#[test]
fn enforce_refuses_above_the_cap() {
    let meta = capped();
    check(&meta, &Value::Int(999), CapPolicy::Enforce).unwrap();
    for value in [1_000, 9_999] {
        let error = check(&meta, &Value::Int(value), CapPolicy::Enforce).unwrap_err();
        assert!(matches!(error, EditError::AboveCap { value: v, cap: 999, .. } if v == value));
    }
}

#[test]
fn storage_only_accepts_above_the_cap() {
    let meta = capped();
    for value in [1_000, 9_999] {
        check(&meta, &Value::Int(value), CapPolicy::StorageOnly).unwrap();
    }
    let error = check(&meta, &Value::Int(10_000), CapPolicy::StorageOnly).unwrap_err();
    assert!(matches!(error, EditError::OutOfRange { value: 10_000, .. }));
}

#[test]
fn the_storage_bound_comes_first() {
    let error = check(&capped(), &Value::Int(10_000), CapPolicy::Enforce).unwrap_err();
    assert!(matches!(
        error,
        EditError::OutOfRange {
            value: 10_000,
            min: 0,
            max: 9_999,
            ..
        }
    ));
}

#[test]
fn a_field_without_a_cap_ignores_the_policy() {
    for policy in BOTH {
        check(&int(0, 9_999), &Value::Int(9_999), policy).unwrap();
    }
}

#[test]
fn the_default_policy_enforces_the_cap() {
    let error = check(&capped(), &Value::Int(1_000), CapPolicy::default()).unwrap_err();
    assert!(matches!(error, EditError::AboveCap { .. }));
}

#[test]
fn exceeds_cap_reads_the_cap_only() {
    let meta = capped();
    assert!(meta.exceeds_cap(&Value::Int(1_000)));
    assert!(!meta.exceeds_cap(&Value::Int(999)));
    assert!(!meta.exceeds_cap(&Value::Int(0)));
    assert!(!meta.exceeds_cap(&Value::Bool(true)));
    assert!(!int(0, 9_999).exceeds_cap(&Value::Int(9_999)));
}

#[test]
fn names_and_tables_do_not_restrict_values() {
    let mut meta = int(1, 12);
    for (value, name) in [(1, "Warrior"), (2, "Thief"), (1, "Duplicate")] {
        meta.names.push(NamedValue { value, name: name.into() });
    }
    meta.tables.push(TableKind::Job);
    for policy in BOTH {
        check(&meta, &Value::Int(7), policy).unwrap();
    }
    assert_eq!(meta.name_of(1), Some("Warrior"));
    assert_eq!(meta.name_of(2), Some("Thief"));
    assert_eq!(meta.name_of(7), None);
}

#[test]
fn new_starts_plain() {
    let meta = FieldMeta::new("Label", Shape::Text);
    assert_eq!(meta.label, "Label");
    assert!(!meta.read_only);
    assert_eq!(meta.cap, None);
    assert_eq!(meta.names, vec![]);
    assert_eq!(meta.tables, vec![]);
}

#[test]
fn shape_reports_its_kind() {
    assert_eq!(Shape::Bool.kind(), ValueKind::Bool);
    assert_eq!(Shape::Text.kind(), ValueKind::Text);
    assert_eq!(Shape::Int { min: 0, max: 1 }.kind(), ValueKind::Int);
}

#[test]
fn a_meta_differs_by_any_one_field() {
    let base = int(0, 9);
    let mut changes: Vec<FieldMeta> = Vec::new();
    let mut label = base.clone();
    label.label = "Other".into();
    changes.push(label);
    let mut shape = base.clone();
    shape.shape = Shape::Int { min: 0, max: 10 };
    changes.push(shape);
    let mut read_only = base.clone();
    read_only.read_only = true;
    changes.push(read_only);
    let mut cap = base.clone();
    cap.cap = Some(5);
    changes.push(cap);
    let mut names = base.clone();
    names.names.push(NamedValue { value: 1, name: "One".into() });
    changes.push(names);
    let mut tables = base.clone();
    tables.tables.push(TableKind::Item);
    changes.push(tables);
    assert_eq!(changes.len(), 6);
    for changed in changes {
        assert_ne!(changed, base);
    }
    assert_eq!(base.clone(), base);
}

fn assert_send_sync_static<T: Send + Sync + 'static>() {}

#[test]
fn meta_types_are_send_sync_and_static() {
    assert_send_sync_static::<FieldMeta>();
    assert_send_sync_static::<Shape>();
    assert_send_sync_static::<NamedValue>();
}
