//! Property tests of the edit contract over generated edits. About half of
//! them are refused, on purpose: a refusal must change nothing.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Model, Snapshot, hp, row};
use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FieldId, Key, Value};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::select;

fn value() -> impl Strategy<Value = Value> {
    prop_oneof![
        (-2_000i64..20_000).prop_map(Value::Int),
        any::<bool>().prop_map(Value::Bool),
        "[a-z]{0,6}".prop_map(Value::Text),
    ]
}

fn scalar_edit() -> BoxedStrategy<Edit> {
    let known = select(Model::sample().ids());
    let field = prop_oneof![known, Just(FieldId::new("nowhere"))];
    let scalar = (field, value()).prop_map(|(field, value)| Edit::set(field, value));
    let count = (0u32..13, value()).prop_map(|(key, value)| Edit::set(row(key).field("count"), value));
    prop_oneof![scalar, count].boxed()
}

fn row_edit() -> BoxedStrategy<Edit> {
    let key = prop_oneof![0u32..13, Just(201u32)];
    let position = prop_oneof![Just(None), (0usize..7).prop_map(Some)];
    let count = prop_oneof![Just(None), (-5i64..120).prop_map(Some)];
    let insert = (key.clone(), position, count).prop_map(|(key, position, count)| {
        let values = count.map(|n| ("count", Value::Int(n))).into_iter().collect();
        Edit::insert(row(key), position, values)
    });
    let remove = key.prop_map(|key| Edit::remove(row(key)));
    prop_oneof![insert, remove].boxed()
}

fn edit() -> BoxedStrategy<Edit> {
    prop_oneof![scalar_edit(), row_edit()].boxed()
}

fn policy() -> impl Strategy<Value = CapPolicy> {
    prop_oneof![Just(CapPolicy::Enforce), Just(CapPolicy::StorageOnly)]
}

fn count_of(snapshot: &Snapshot, key: u32) -> Option<i64> {
    snapshot.rows.iter().find(|(k, _)| *k == key).map(|(_, count)| *count)
}

/// The value a `Set` replaces.
fn previous(snapshot: &Snapshot, field: &FieldId) -> Option<Value> {
    if let [Key::Name("inventory"), Key::Id(key), Key::Name("count")] = field.keys() {
        return count_of(snapshot, *key).map(Value::Int);
    }
    snapshot.scalars.get(field).cloned()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn apply_then_the_inverse_restores_the_model(edits in vec(edit(), 0..20), policy in policy()) {
        let mut model = Model::sample();
        for edit in &edits {
            let before = model.snapshot();
            if let Ok(inverse) = model.apply(edit, policy) {
                let applied = model.snapshot();
                prop_assert!(model.apply(&inverse, CapPolicy::StorageOnly).is_ok());
                prop_assert_eq!(model.snapshot(), before);
                prop_assert!(model.apply(edit, CapPolicy::StorageOnly).is_ok());
                prop_assert_eq!(model.snapshot(), applied);
            } else {
                prop_assert_eq!(model.snapshot(), before);
            }
        }
    }

    #[test]
    fn the_inverse_has_the_documented_shape(edits in vec(edit(), 0..20), policy in policy()) {
        let mut model = Model::sample();
        for edit in &edits {
            let before = model.snapshot();
            let Ok(inverse) = model.apply(edit, policy) else { continue };
            match (edit, inverse) {
                (Edit::Set { field, .. }, Edit::Set { field: back, value }) => {
                    prop_assert_eq!(&back, field);
                    prop_assert_eq!(Some(value), previous(&before, field));
                }
                (Edit::Insert { row, .. }, Edit::Remove { row: back }) => prop_assert_eq!(&back, row),
                (Edit::Remove { row }, Edit::Insert { row: back, position, values }) => {
                    let index = before.rows.iter().position(|(key, _)| *key == row.key());
                    let count = count_of(&before, row.key()).unwrap();
                    prop_assert_eq!(&back, row);
                    prop_assert_eq!(position, index);
                    prop_assert_eq!(values, vec![("count", Value::Int(count))]);
                }
                (edit, inverse) => prop_assert!(false, "{edit:?} answered {inverse:?}"),
            }
        }
    }

    #[test]
    fn change_sets_are_atomic(edits in vec(edit(), 0..8), policy in policy()) {
        let mut model = Model::sample();
        let before = model.snapshot();
        let mut replay = model.clone();
        let first_error = edits.iter().find_map(|edit| replay.apply(edit, policy).err());
        let set: ChangeSet = edits.into_iter().collect();
        match set.apply_with(policy, |edit, p| model.apply(edit, p)) {
            Ok(_) => prop_assert!(first_error.is_none()),
            Err(error) => {
                let rollback_failed = matches!(error, EditError::RollbackFailed { .. });
                prop_assert!(!rollback_failed);
                prop_assert_eq!(format!("{error:?}"), format!("{:?}", first_error.unwrap()));
                prop_assert_eq!(model.snapshot(), before);
            }
        }
    }

    #[test]
    fn change_sets_invert_and_redo(edits in vec(edit(), 0..8), policy in policy()) {
        let mut model = Model::sample();
        let before = model.snapshot();
        let set: ChangeSet = edits.into_iter().collect();
        if let Ok(inverse) = set.apply_with(policy, |edit, p| model.apply(edit, p)) {
            let after = model.snapshot();
            let redo = inverse.apply_with(CapPolicy::StorageOnly, |edit, p| model.apply(edit, p)).unwrap();
            prop_assert_eq!(model.snapshot(), before);
            redo.apply_with(CapPolicy::StorageOnly, |edit, p| model.apply(edit, p)).unwrap();
            prop_assert_eq!(model.snapshot(), after);
        }
    }

    #[test]
    fn the_policy_decides_the_cap_and_nothing_else(number in -2_000i64..20_000) {
        let edit = Edit::set(hp(1), Value::Int(number));
        let enforced = Model::sample().apply(&edit, CapPolicy::Enforce);
        let verdict = match &enforced {
            Ok(_) => "ok",
            Err(EditError::AboveCap { .. }) => "above-cap",
            Err(EditError::OutOfRange { .. }) => "out-of-range",
            Err(_) => "other",
        };
        let expected = match number {
            0..=999 => "ok",
            1_000..=9_999 => "above-cap",
            _ => "out-of-range",
        };
        prop_assert_eq!(verdict, expected);
        let expert = Model::sample().apply(&edit, CapPolicy::StorageOnly);
        prop_assert_eq!(expert.is_ok(), (0..=9_999).contains(&number));
    }

    #[test]
    fn an_expert_edit_can_always_be_undone(above in 1_000i64..=9_999, below in 0i64..=999) {
        let mut model = Model::sample();
        model.apply(&Edit::set(hp(1), Value::Int(above)), CapPolicy::StorageOnly).unwrap();
        let expert = model.snapshot();

        let inverse = model.apply(&Edit::set(hp(1), Value::Int(below)), CapPolicy::Enforce).unwrap();
        prop_assert!(model.apply(&inverse, CapPolicy::StorageOnly).is_ok());
        prop_assert_eq!(model.snapshot(), expert.clone());

        let set: ChangeSet =
            [Edit::set(hp(1), Value::Int(below)), Edit::set(hp(2), Value::Int(99_999))].into_iter().collect();
        let result = set.apply_with(CapPolicy::Enforce, |edit, p| model.apply(edit, p));
        let refused = matches!(result, Err(EditError::OutOfRange { .. }));
        prop_assert!(refused);
        prop_assert_eq!(model.snapshot(), expert);
    }
}
