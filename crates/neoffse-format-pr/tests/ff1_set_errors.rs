//! The refusals of the raw `apply` for `Set`: errors, bounds, caps, tables
//! and the stored-value checks. A failed `Set` changes nothing.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{SlotData, character_field, model, standard_model};
use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FieldId, Value};
use neoffse_format_pr::document::DocumentError;
use neoffse_format_pr::ff1::names;

const ENFORCE: CapPolicy = CapPolicy::Enforce;
const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;
const STORAGE_MAX: i64 = 2_147_483_647;

fn gil() -> FieldId {
    FieldId::new(names::GIL)
}

fn count(list: &'static str, key: u32) -> FieldId {
    FieldId::new(list).with_id(key).with_name(names::COUNT)
}

fn slot(slot: u32) -> FieldId {
    character_field(1, names::EQUIPMENT).with_id(slot)
}

/// `Set` on a fresh standard model.
fn try_set(field: &FieldId, value: i64, policy: CapPolicy) -> Result<Edit, EditError> {
    standard_model().apply(&Edit::set(field.clone(), Value::Int(value)), policy)
}

#[test]
fn unknown_fields_and_missing_rows() {
    let mut model = standard_model();
    let unknown = [
        character_field(1, "nope"),
        character_field(1, names::CHARGES).with_id(0),
        character_field(1, names::CHARGES).with_id(9),
        character_field(1, names::EQUIPMENT).with_id(6),
        character_field(1, names::EQUIPMENT).with_id(1).with_name("nope"),
        FieldId::new(names::INVENTORY).with_id(89).with_name(names::NAME),
        FieldId::new("nothing"),
        FieldId::new(names::CHARACTER).with_id(1),
    ];
    for field in unknown {
        let result = model.apply(&Edit::set(field.clone(), Value::Int(1)), ENFORCE);
        assert!(matches!(result, Err(EditError::UnknownField { field: named }) if named == field), "{field}");
    }
    let absent = [
        (character_field(9, names::NAME), FieldId::new(names::CHARACTER).with_id(9)),
        (count(names::INVENTORY, 999), FieldId::new(names::INVENTORY).with_id(999)),
        (count(names::KEY_ITEMS, 89), FieldId::new(names::KEY_ITEMS).with_id(89)),
    ];
    for (field, want) in absent {
        let result = model.apply(&Edit::set(field.clone(), Value::Int(1)), ENFORCE);
        assert!(matches!(result, Err(EditError::MissingRow { row }) if row == want), "{field}");
    }
    let mut data = SlotData::standard();
    data.characters[0].charges.retain(|(level, _)| *level != 3);
    data.characters[0].equipment.retain(|equip| equip.key != 4);
    let mut model = model_of(&data);
    for field in [character_field(1, names::CHARGES).with_id(3), slot(4), slot(4).with_name(names::COUNT)] {
        let result = model.apply(&Edit::set(field.clone(), Value::Int(1)), ENFORCE);
        assert!(matches!(result, Err(EditError::UnknownField { field: named }) if named == field), "{field}");
    }
}

fn model_of(data: &SlotData) -> neoffse_format_pr::ff1::Ff1Model {
    model(&data.render())
}

#[test]
fn wrong_type_is_refused() {
    let mut model = standard_model();
    let cases = [
        (character_field(1, names::LEVEL), Value::Text("5".to_owned())),
        (character_field(1, names::NAME), Value::Int(5)),
        (gil(), Value::Bool(true)),
    ];
    for (field, value) in cases {
        for policy in [ENFORCE, STORAGE_ONLY] {
            let result = model.apply(&Edit::set(field.clone(), value.clone()), policy);
            assert!(matches!(result, Err(EditError::WrongType { .. })), "{field}");
        }
    }
}

#[test]
fn storage_bounds_hold_under_both_policies() {
    let power = character_field(1, names::POWER);
    for policy in [ENFORCE, STORAGE_ONLY] {
        for value in [-1, STORAGE_MAX + 1] {
            let result = try_set(&power, value, policy);
            assert!(matches!(result, Err(EditError::OutOfRange { min: 0, max: STORAGE_MAX, .. })));
        }
        let result = try_set(&count(names::INVENTORY, 89), 0, policy);
        assert!(matches!(result, Err(EditError::OutOfRange { min: 1, .. })));
    }
    // Magic has no game cap, so Enforce takes the storage maximum.
    assert!(try_set(&character_field(1, names::MAGIC), STORAGE_MAX, ENFORCE).is_ok());
    assert!(try_set(&power, STORAGE_MAX, STORAGE_ONLY).is_ok());
}

#[test]
fn the_level_storage_bound_is_the_game_cap() {
    let level = character_field(1, names::LEVEL);
    for policy in [ENFORCE, STORAGE_ONLY] {
        assert!(try_set(&level, 99, policy).is_ok());
        for value in [-1, 100, STORAGE_MAX] {
            let result = try_set(&level, value, policy);
            assert!(matches!(result, Err(EditError::OutOfRange { min: 0, max: 99, .. })), "{value}");
        }
    }
}

#[test]
fn caps_under_enforce_and_storage_only() {
    let cases = [
        (gil(), 999_999),
        (character_field(1, names::EXP), 9_999_999),
        (character_field(1, names::HP), 999),
        (character_field(1, names::POWER), 99),
        (character_field(1, names::CHARGES).with_id(1), 9),
        (count(names::INVENTORY, 100), 99),
    ];
    for (field, cap) in cases {
        assert!(try_set(&field, cap, ENFORCE).is_ok(), "{field}");
        let result = try_set(&field, cap + 1, ENFORCE);
        assert!(
            matches!(result, Err(EditError::AboveCap { cap: found, value, .. }) if found == cap && value == cap + 1),
            "{field}"
        );
        assert!(try_set(&field, cap + 1, STORAGE_ONLY).is_ok(), "{field}");
    }
}

#[test]
fn a_value_past_the_bound_and_the_cap_is_out_of_range() {
    let result = try_set(&character_field(1, names::LEVEL), STORAGE_MAX + 1, ENFORCE);
    assert!(matches!(result, Err(EditError::OutOfRange { .. })));
}

#[test]
fn the_inverse_of_an_above_cap_edit_needs_storage_only() {
    let gil = gil();
    let mut model = standard_model();
    model.apply(&Edit::set(gil.clone(), Value::Int(1_500_000)), STORAGE_ONLY).unwrap();
    let inverse = model.apply(&Edit::set(gil.clone(), Value::Int(50)), ENFORCE).unwrap();
    assert_eq!(inverse, Edit::set(gil.clone(), Value::Int(1_500_000)));
    let result = model.apply(&inverse, ENFORCE);
    assert!(matches!(result, Err(EditError::AboveCap { cap: 999_999, .. })));
    assert_eq!(model.get(&gil).unwrap(), Value::Int(50));
    model.apply(&inverse, STORAGE_ONLY).unwrap();
    assert_eq!(model.get(&gil).unwrap(), Value::Int(1_500_000));
}

#[test]
fn jobs_are_checked_against_the_job_table_under_enforce() {
    let job = character_field(1, names::JOB);
    assert!(try_set(&job, 12, ENFORCE).is_ok());
    for id in [13, 0] {
        assert!(matches!(try_set(&job, id, ENFORCE), Err(EditError::UnknownId { id: found, .. }) if found == id));
    }
    assert!(try_set(&job, 13, STORAGE_ONLY).is_ok());
}

#[test]
fn equipment_ids_must_belong_to_the_slot_table_under_enforce() {
    let accepted = [(2, 165), (1, 62), (2, 130), (3, 131), (4, 132), (5, 133)];
    for (number, id) in accepted {
        assert!(try_set(&slot(number), id, ENFORCE).is_ok(), "slot {number} id {id}");
    }
    let refused = [(1, 165), (2, 62), (1, 45), (1, 999), (3, 0)];
    for (number, id) in refused {
        let result = try_set(&slot(number), id, ENFORCE);
        assert!(
            matches!(result, Err(EditError::UnknownId { id: found, .. }) if found == id),
            "slot {number} id {id}"
        );
    }
    assert!(try_set(&slot(1), 999, STORAGE_ONLY).is_ok());
    assert!(try_set(&slot(3), 0, STORAGE_ONLY).is_ok());
}

#[test]
fn a_stored_value_outside_the_storage_bound_refuses_the_edit() {
    let mut data = SlotData::standard();
    data.characters[0].level = "-5".to_owned();
    let text = data.render();
    let mut model = model(&text);
    let result = model.apply(&Edit::set(character_field(1, names::LEVEL), Value::Int(10)), ENFORCE);
    assert!(matches!(result, Err(EditError::OutOfRange { value: -5, .. })));
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn a_stored_non_integer_refuses_the_edit() {
    let mut data = SlotData::standard();
    data.characters[0].level = "31.0".to_owned();
    let text = data.render();
    let mut model = model(&text);
    let result = model.apply(&Edit::set(character_field(1, names::LEVEL), Value::Int(10)), ENFORCE);
    let Err(EditError::Format { source, .. }) = result else {
        panic!("no Format error")
    };
    assert!(source.downcast_ref::<DocumentError>().is_some());
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn a_failed_set_leaves_the_text_untouched() {
    let mut data = SlotData::standard();
    data.spaced = true;
    let text = data.render();
    let mut model = model(&text);
    let level = character_field(1, names::LEVEL);
    let cases = [
        (FieldId::new("nothing"), Value::Int(1), ENFORCE),
        (character_field(9, names::LEVEL), Value::Int(1), ENFORCE),
        (level.clone(), Value::Text("x".to_owned()), ENFORCE),
        (level.clone(), Value::Int(-1), STORAGE_ONLY),
        (level.clone(), Value::Int(100), ENFORCE),
        (character_field(1, names::JOB), Value::Int(13), ENFORCE),
        (slot(1), Value::Int(165), ENFORCE),
    ];
    for (field, value, policy) in cases {
        assert!(model.apply(&Edit::set(field.clone(), value), policy).is_err(), "{field}");
        assert_eq!(model.document().to_json(), text, "{field}");
    }
}

#[test]
fn a_change_set_applies_all_or_none_through_the_model() {
    let text = SlotData::standard().render();
    let level = character_field(1, names::LEVEL);
    let bad: ChangeSet = [Edit::set(level.clone(), Value::Int(50)), Edit::set(gil(), Value::Int(9_999_999))]
        .into_iter()
        .collect();
    let mut model = model(&text);
    let result = bad.apply_with(CapPolicy::Enforce, |edit, policy| model.apply(edit, policy));
    assert!(matches!(result, Err(EditError::AboveCap { cap: 999_999, .. })));
    assert_eq!(model.document().to_json(), text);

    let good: ChangeSet = [Edit::set(level.clone(), Value::Int(50)), Edit::set(gil(), Value::Int(900_000))]
        .into_iter()
        .collect();
    let inverse = good.apply_with(CapPolicy::Enforce, |edit, policy| model.apply(edit, policy)).unwrap();
    assert_eq!(model.get(&level).unwrap(), Value::Int(50));
    inverse.apply_with(CapPolicy::StorageOnly, |edit, policy| model.apply(edit, policy)).unwrap();
    assert_eq!(model.document().to_json(), text);
}
