//! `ChangeSet`: order, inverse, rollback and the policy each call receives.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Model, hp, insert, int, remove, text};
use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FieldId};

type Log = Vec<(Edit, CapPolicy)>;

/// Applies `set` to `model` and records every call `apply_with` makes.
fn run(model: &mut Model, set: &ChangeSet, policy: CapPolicy) -> (Result<ChangeSet, EditError>, Log) {
    let mut log = Vec::new();
    let result = set.apply_with(policy, |edit, p| {
        log.push((edit.clone(), p));
        model.apply(edit, p)
    });
    (result, log)
}

fn field(name: &'static str) -> FieldId {
    FieldId::new(name)
}

fn targets(log: &[(Edit, CapPolicy)]) -> Vec<String> {
    log.iter().map(|(edit, _)| edit.target().to_string()).collect()
}

#[test]
fn an_empty_set_applies_nothing() {
    let mut calls = 0;
    let result = ChangeSet::new()
        .apply_with(CapPolicy::Enforce, |_, _| {
            calls += 1;
            Err(EditError::UnknownField { field: field("x") })
        })
        .unwrap();
    assert!(result.is_empty());
    assert_eq!(calls, 0);
}

#[test]
fn a_set_collects_in_order() {
    let edits = [int(hp(1), 1), int(hp(2), 2), int(field("gil"), 3)];
    let mut pushed = ChangeSet::new();
    for edit in edits.clone() {
        pushed.push(edit);
    }
    let collected: ChangeSet = edits.clone().into_iter().collect();
    assert_eq!(pushed, collected);
    assert_eq!(pushed.len(), 3);
    assert!(!pushed.is_empty());
    assert_eq!(pushed.edits(), edits);
}

#[test]
fn edits_apply_in_order() {
    let mut model = Model::sample();
    let set: ChangeSet = [int(hp(1), 10), int(hp(1), 20)].into_iter().collect();
    let (result, log) = run(&mut model, &set, CapPolicy::Enforce);
    result.unwrap();
    assert_eq!(model.value(&hp(1)), Some(&neoffse_core::Value::Int(20)));
    let seen: Vec<Edit> = log.into_iter().map(|(edit, _)| edit).collect();
    assert_eq!(seen, set.edits());
}

#[test]
fn the_inverse_runs_in_reverse_order() {
    let mut model = Model::sample();
    let before = model.snapshot();
    let set: ChangeSet = [int(hp(1), 10), int(hp(1), 20)].into_iter().collect();
    let inverse = run(&mut model, &set, CapPolicy::Enforce).0.unwrap();
    let expected: ChangeSet = [int(hp(1), 10), int(hp(1), 100)].into_iter().collect();
    assert_eq!(inverse, expected);
    run(&mut model, &inverse, CapPolicy::StorageOnly).0.unwrap();
    assert_eq!(model.value(&hp(1)), Some(&neoffse_core::Value::Int(100)));
    assert_eq!(model.snapshot(), before);
}

#[test]
fn for_set_edits_the_inverse_of_the_inverse_is_the_original_set() {
    let mut model = Model::sample();
    let set: ChangeSet = [int(hp(1), 10), int(hp(1), 20)].into_iter().collect();
    let inverse = run(&mut model, &set, CapPolicy::Enforce).0.unwrap();
    let redo = run(&mut model, &inverse, CapPolicy::StorageOnly).0.unwrap();
    assert_eq!(redo, set);
}

#[test]
fn a_failing_edit_rolls_back_the_applied_ones() {
    let mut model = Model::sample();
    let before = model.snapshot();
    let set: ChangeSet = [int(hp(1), 10), int(field("gil"), 5), int(hp(2), 99_999)].into_iter().collect();
    let (result, log) = run(&mut model, &set, CapPolicy::Enforce);
    assert!(matches!(
        result,
        Err(EditError::OutOfRange { ref field, value: 99_999, min: 0, max: 9_999 }) if *field == hp(2)
    ));
    assert_eq!(model.snapshot(), before);
    assert_eq!(targets(&log), ["character.1.hp", "gil", "character.2.hp", "gil", "character.1.hp"]);
    assert_eq!(log[3].0, int(field("gil"), 0));
    assert_eq!(log[4].0, int(hp(1), 100));
}

#[test]
fn a_failure_at_the_first_edit_applies_and_undoes_nothing() {
    let mut model = Model::sample();
    let before = model.snapshot();
    let set: ChangeSet = [int(hp(2), 99_999), int(hp(1), 10)].into_iter().collect();
    let (result, log) = run(&mut model, &set, CapPolicy::Enforce);
    assert!(result.is_err());
    assert_eq!(log.len(), 1);
    assert_eq!(model.snapshot(), before);
}

#[test]
fn a_failure_at_the_last_edit_undoes_every_earlier_edit() {
    let mut model = Model::sample();
    let before = model.snapshot();
    let name = FieldId::new("character").with_id(1).with_name("name");
    let set: ChangeSet = [int(field("gil"), 1), int(hp(1), 1), int(hp(2), 1), text(name, "Z"), int(hp(2), 99_999)]
        .into_iter()
        .collect();
    let (result, log) = run(&mut model, &set, CapPolicy::Enforce);
    assert!(result.is_err());
    assert_eq!(
        targets(&log),
        [
            "gil",
            "character.1.hp",
            "character.2.hp",
            "character.1.name",
            "character.2.hp",
            "character.1.name",
            "character.2.hp",
            "character.1.hp",
            "gil",
        ]
    );
    assert_eq!(model.snapshot(), before);
}

#[test]
fn the_first_error_is_the_one_returned() {
    let mut model = Model::sample();
    let set: ChangeSet = [int(hp(1), 10), int(field("nowhere"), 1), int(hp(2), 99_999)].into_iter().collect();
    let (result, log) = run(&mut model, &set, CapPolicy::Enforce);
    assert!(matches!(result, Err(EditError::UnknownField { ref field }) if field.to_string() == "nowhere"));
    assert!(!targets(&log).iter().any(|t| t == "character.2.hp"));
}

#[test]
fn a_failed_undo_reports_both_errors() {
    let undo = int(field("x"), 1);
    let set: ChangeSet = [int(field("a"), 1), int(field("b"), 2)].into_iter().collect();
    let mut call = 0;
    let result = set.apply_with(CapPolicy::Enforce, |_, _| {
        call += 1;
        match call {
            1 => Ok(undo.clone()),
            2 => Err(EditError::ReadOnly { field: field("b") }),
            _ => Err(EditError::UnknownField { field: field("x") }),
        }
    });
    let Err(EditError::RollbackFailed {
        field: failed,
        source,
        rollback,
    }) = result
    else {
        panic!("expected RollbackFailed");
    };
    assert_eq!(&failed, undo.target());
    assert!(matches!(*source, EditError::ReadOnly { .. }));
    assert!(matches!(*rollback, EditError::UnknownField { .. }));
}

#[test]
fn an_undo_stops_at_the_first_failed_undo() {
    let set: ChangeSet = [int(field("a"), 1), int(field("b"), 1), int(field("c"), 1), int(field("d"), 1)]
        .into_iter()
        .collect();
    let mut log: Vec<String> = Vec::new();
    let mut call = 0;
    let result = set.apply_with(CapPolicy::Enforce, |edit, _| {
        call += 1;
        log.push(edit.target().to_string());
        match call {
            1..=3 => Ok(int(field(["ua", "ub", "uc"][call - 1]), 0)),
            4 => Err(EditError::ReadOnly { field: field("d") }),
            5 => Ok(int(field("unused"), 0)),
            _ => Err(EditError::UnknownField { field: field("ub") }),
        }
    });
    assert!(matches!(result, Err(EditError::RollbackFailed { .. })));
    assert_eq!(log, ["a", "b", "c", "d", "uc", "ub"]);
}

#[test]
fn the_closure_runs_once_per_edit_on_success() {
    let mut model = Model::sample();
    let set: ChangeSet = [int(hp(1), 1), int(hp(2), 2), int(field("gil"), 3)].into_iter().collect();
    let (result, log) = run(&mut model, &set, CapPolicy::Enforce);
    assert_eq!(result.unwrap().len(), 3);
    assert_eq!(log.len(), 3);
}

#[test]
fn a_no_op_edit_returns_itself_as_its_inverse() {
    let mut model = Model::sample();
    let edit = int(field("gil"), 0);
    let inverse = model.apply(&edit, CapPolicy::Enforce).unwrap();
    assert_eq!(inverse, edit);
}

#[test]
fn forward_edits_get_the_callers_policy_and_undo_gets_storage_only() {
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        let mut model = Model::sample();
        let set: ChangeSet = [int(hp(1), 10), int(field("gil"), 5), int(hp(2), 99_999)].into_iter().collect();
        let (result, log) = run(&mut model, &set, policy);
        assert!(result.is_err());
        let policies: Vec<CapPolicy> = log.iter().map(|(_, p)| *p).collect();
        assert_eq!(policies[..3], [policy; 3]);
        assert_eq!(policies[3..], [CapPolicy::StorageOnly; 2]);
    }
}

fn mixed(last: i64) -> ChangeSet {
    [insert(5, Some(0), Some(4)), remove(7), int(hp(1), 10), int(hp(2), last)].into_iter().collect()
}

#[test]
fn a_mixed_set_rolls_back_rows_and_values() {
    let mut model = Model::sample();
    let before = model.snapshot();
    let (result, _) = run(&mut model, &mixed(99_999), CapPolicy::Enforce);
    assert!(result.is_err());
    assert_eq!(model.snapshot(), before);
    assert_eq!(model.rows(), [3, 7, 9]);
}

#[test]
fn a_mixed_set_inverts_to_the_earlier_state_and_redo_reaches_the_later_one() {
    let mut model = Model::sample();
    let before = model.snapshot();
    let inverse = run(&mut model, &mixed(20), CapPolicy::Enforce).0.unwrap();
    let after = model.snapshot();
    assert_eq!(model.rows(), [5, 3, 9]);
    let redo = run(&mut model, &inverse, CapPolicy::StorageOnly).0.unwrap();
    assert_eq!(model.snapshot(), before);
    run(&mut model, &redo, CapPolicy::StorageOnly).0.unwrap();
    assert_eq!(model.snapshot(), after);
}
