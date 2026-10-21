//! `CapPolicy`: what each policy accepts and what an inverse always may.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Model, hp, insert, int};
use neoffse_core::{CapPolicy, EditError, FieldId};

#[test]
fn enforce_refuses_above_the_cap() {
    let mut model = Model::sample();
    let before = model.snapshot();
    let error = model.apply(&int(hp(1), 1_000), CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::AboveCap { value: 1_000, cap: 999, .. }));
    assert_eq!(model.snapshot(), before);
    model.apply(&int(hp(1), 999), CapPolicy::Enforce).unwrap();
}

#[test]
fn storage_only_accepts_above_the_cap() {
    for value in [1_000, 9_999] {
        let mut model = Model::sample();
        model.apply(&int(hp(1), value), CapPolicy::StorageOnly).unwrap();
    }
}

#[test]
fn the_storage_bound_comes_first() {
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        let mut model = Model::sample();
        let error = model.apply(&int(hp(1), 10_000), policy).unwrap_err();
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
}

#[test]
fn a_field_without_a_cap_ignores_the_policy() {
    let mut model = Model::sample();
    model.apply(&int(FieldId::new("gil"), 999_999), CapPolicy::Enforce).unwrap();
}

#[test]
fn an_inverse_above_the_cap_applies_under_storage_only() {
    let mut model = Model::sample();
    model.apply(&int(hp(1), 5_000), CapPolicy::StorageOnly).unwrap();
    let after_expert = model.snapshot();
    let inverse = model.apply(&int(hp(1), 10), CapPolicy::Enforce).unwrap();
    assert_eq!(inverse, int(hp(1), 5_000));

    let mut enforced = model.clone();
    let error = enforced.apply(&inverse, CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::AboveCap { value: 5_000, cap: 999, .. }));

    model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
    assert_eq!(model.snapshot(), after_expert);
}

#[test]
fn insert_values_follow_the_same_checks_under_both_policies() {
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        let mut model = Model::sample();
        model.apply(&insert(5, None, Some(99)), policy).unwrap();
        let error = model.apply(&insert(6, None, Some(100)), policy).unwrap_err();
        assert!(matches!(error, EditError::OutOfRange { value: 100, .. }));
    }
}

#[test]
fn the_default_policy_is_enforce() {
    let mut model = Model::sample();
    let error = model.apply(&int(hp(1), 1_000), CapPolicy::default()).unwrap_err();
    assert!(matches!(error, EditError::AboveCap { .. }));
}
