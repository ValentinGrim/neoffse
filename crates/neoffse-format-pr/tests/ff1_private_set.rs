//! The raw `apply` of the FF I model on the private slot files. The folder
//! is read-only input: edits happen in memory and the tests write nothing.
//! A failure names the label and a field path, never a value.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::changed_span;
use common::ff1_real::{RealSlot, must_some, slots};
use neoffse_core::{CapPolicy, Edit, FieldId, Shape, Value};
use neoffse_format_pr::container::{decode, encode};
use neoffse_format_pr::ff1::{Ff1Model, names};

fn int(model: &mut Ff1Model, label: &str, field: &FieldId) -> i64 {
    match model.get(field) {
        Ok(Value::Int(value)) => value,
        _ => panic!("{label}: {field} does not read as an integer"),
    }
}

/// `old - 1`, or 1 when `old` is 0.
fn lower(old: i64) -> i64 {
    if old == 0 { 1 } else { old - 1 }
}

/// Sets `field` to `old - 1`, encodes, decodes, and checks that only digits
/// changed and that the new value reads back.
fn assert_digit_edit(slot: &RealSlot, field: &FieldId) {
    let label = &slot.label;
    let mut model = slot.model();
    let value = lower(int(&mut model, label, field));
    let Ok(_) = model.apply(&Edit::set(field.clone(), Value::Int(value)), CapPolicy::Enforce) else {
        panic!("{label}: {field} refuses the edit")
    };
    let json = model.document().to_json();
    let Ok(decoded) = decode(&slot.bytes) else { panic!("{label}: decode failed") };
    let Ok(bytes) = encode(&json, &decoded.container) else {
        panic!("{label}: encode failed")
    };
    let Ok(again) = decode(&bytes) else { panic!("{label}: second decode failed") };
    assert!(again.json == json, "{label}: {field} changes in the round trip");
    let (was, now) = changed_span(&slot.json, &again.json);
    for side in [was, now] {
        assert!(
            !side.is_empty() && side.bytes().all(|byte| byte.is_ascii_digit()),
            "{label}: {field} changed more than digits"
        );
    }
    let mut rebuilt = RealSlot {
        label: label.clone(),
        name: slot.name.clone(),
        bytes,
        json: again.json,
    }
    .model();
    assert!(int(&mut rebuilt, label, field) == value, "{label}: {field} does not read back");
}

#[test]
fn real_gil_edit_changes_only_digits() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        assert_digit_edit(&slot, &FieldId::new(names::GIL));
    }
}

#[test]
fn real_character_edits_change_only_digits() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model();
        let Ok(ids) = model.rows(&FieldId::new(names::CHARACTER)) else {
            panic!("{}: character rows do not read", slot.label)
        };
        let Some(first) = ids.first() else { continue };
        for field in names::character_fields(*first) {
            let meta = must_some(model.field_meta(&field), &format!("{}: {field} has no meta", slot.label));
            if matches!(meta.shape, Shape::Int { .. }) && meta.tables.is_empty() {
                assert_digit_edit(&slot, &field);
            }
        }
    }
}

#[test]
fn real_edit_then_inverse_restores_the_text() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model();
        let Ok(ids) = model.rows(&FieldId::new(names::CHARACTER)) else {
            panic!("{}: character rows do not read", slot.label)
        };
        let mut fields = vec![FieldId::new(names::GIL)];
        if let Some(first) = ids.first() {
            let character = FieldId::new(names::CHARACTER).with_id(*first);
            fields.push(character.clone().with_name(names::LEVEL));
            fields.push(character.with_name(names::NAME));
        }
        for field in fields {
            let mut model = slot.model();
            let Ok(old) = model.get(&field) else {
                panic!("{}: {field} does not read", slot.label)
            };
            let value = match old {
                Value::Int(old) => Value::Int(lower(old)),
                _ => Value::Text("Probe".to_owned()),
            };
            let Ok(inverse) = model.apply(&Edit::set(field.clone(), value), CapPolicy::Enforce) else {
                panic!("{}: {field} refuses the edit", slot.label)
            };
            let Ok(_) = model.apply(&inverse, CapPolicy::StorageOnly) else {
                panic!("{}: {field} refuses the inverse", slot.label)
            };
            assert!(model.document().to_json() == slot.json, "{}: {field} does not restore", slot.label);
        }
    }
}
