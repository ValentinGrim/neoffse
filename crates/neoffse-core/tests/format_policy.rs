//! The policy of a caller reaches the checks of a document: `apply_all`
//! forwards it, and the ID table gate depends on it.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::fake::{add, bytes, doc, hp, row};
use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, Value};

#[test]
fn apply_all_keeps_the_callers_policy() {
    let changes: ChangeSet = [Edit::set(hp(1), Value::Int(5_000))].into_iter().collect();

    let mut enforced = doc();
    let view = enforced.view().unwrap();
    let encoding = bytes(&*enforced);
    let error = enforced.apply_all(&changes, CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::AboveCap { value: 5_000, cap: 999, .. }), "{error:?}");
    assert_eq!(enforced.view().unwrap(), view);
    assert_eq!(bytes(&*enforced), encoding);

    let mut expert = doc();
    expert.apply_all(&changes, CapPolicy::StorageOnly).unwrap();
    assert_eq!(expert.view().unwrap().find(&hp(1)).unwrap().1.value, Value::Int(5_000));
}

#[test]
fn a_key_outside_the_id_table_is_refused_under_enforce_only() {
    let mut document = doc();
    let encoding = bytes(&*document);
    let error = document.apply(&add(100, None, 1), CapPolicy::Enforce).unwrap_err();
    assert!(matches!(
        error,
        EditError::UnknownId { ref field, id: 100 } if *field == *row(100).id()
    ));
    assert_eq!(bytes(&*document), encoding);

    let inverse = document.apply(&add(100, None, 1), CapPolicy::StorageOnly).unwrap();
    assert_eq!(inverse, Edit::remove(row(100)));
    assert_ne!(bytes(&*document), encoding);
}

#[test]
fn a_key_outside_the_storage_bound_is_out_of_range_under_both_policies() {
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        let mut document = doc();
        let error = document.apply(&add(201, None, 1), policy).unwrap_err();
        assert!(
            matches!(
                error,
                EditError::OutOfRange {
                    value: 201,
                    min: 1,
                    max: 200,
                    ..
                }
            ),
            "{error:?}"
        );
    }
}
