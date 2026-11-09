//! The raw `apply` of FF II: the order of checks for `Set`, the item-list
//! `Insert` and `Remove`, and the rule that a raw edit holds no game rule. A
//! failed call changes nothing.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_field;
use common::character_path;
use common::ff2_save::{SlotData, ids, key_items, model, standard_model, tables};
use common::{ENFORCE, STORAGE_MAX, STORAGE_ONLY, count, gil, row, slot};
use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FieldId, TableKind, Value};
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Model, names};

fn skill(key: u32) -> FieldId {
    character_field(1, names::SKILL).with_id(key)
}

/// `Set` on a fresh standard model.
fn try_set(field: &FieldId, value: i64, policy: CapPolicy) -> Result<Edit, EditError> {
    standard_model().apply(&Edit::set(field.clone(), Value::Int(value)), policy)
}

fn model_of(data: &SlotData) -> Ff2Model {
    model(&data.render())
}

#[test]
fn unknown_fields_and_missing_rows() {
    let mut model = standard_model();
    let unknown = [
        character_field(1, "nope"),
        character_field(1, names::SKILL).with_id(10),
        character_field(1, names::EQUIPMENT).with_id(6),
        character_field(1, names::EQUIPMENT).with_id(1).with_name("nope"),
        FieldId::new(names::INVENTORY).with_id(60).with_name(names::NAME),
        FieldId::new("nothing"),
        FieldId::new(names::CHARACTER).with_id(1),
    ];
    for field in unknown {
        let result = model.apply(&Edit::set(field.clone(), Value::Int(1)), ENFORCE);
        assert!(matches!(result, Err(EditError::UnknownField { field: named }) if named == field), "{field}");
    }
    let absent = [
        (character_field(9, names::NAME), FieldId::new(names::CHARACTER).with_id(9)),
        (count(names::INVENTORY, 999_999), FieldId::new(names::INVENTORY).with_id(999_999)),
        (count(names::KEY_ITEMS, 999_999), FieldId::new(names::KEY_ITEMS).with_id(999_999)),
    ];
    for (field, want) in absent {
        let result = model.apply(&Edit::set(field.clone(), Value::Int(1)), ENFORCE);
        assert!(matches!(result, Err(EditError::MissingRow { row }) if row == want), "{field}");
    }
    let mut data = SlotData::standard();
    data.characters[0].skills.retain(|(key, _)| *key != 3);
    data.characters[0].equipment.retain(|equip| equip.key != 4);
    let mut model = model_of(&data);
    for field in [skill(3), slot(1, 4), slot(1, 4).with_name(names::COUNT)] {
        let result = model.apply(&Edit::set(field.clone(), Value::Int(1)), ENFORCE);
        assert!(matches!(result, Err(EditError::UnknownField { field: named }) if named == field), "{field}");
    }
}

#[test]
fn wrong_type_is_refused() {
    let mut model = standard_model();
    let cases = [
        (character_field(1, names::HP), Value::Text("5".to_owned())),
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
        let result = try_set(&count(names::INVENTORY, 60), 0, policy);
        assert!(matches!(result, Err(EditError::OutOfRange { min: 1, .. })));
    }
    // A gain has no game cap of its own: the cap depends on the character.
    assert!(try_set(&power, STORAGE_MAX, ENFORCE).is_ok());
    assert!(try_set(&power, STORAGE_MAX, STORAGE_ONLY).is_ok());
}

#[test]
fn caps_under_enforce_and_storage_only() {
    let cases = [
        (gil(), 9_999_999),
        (character_field(1, names::HP), 9_999),
        (character_field(1, names::MP), 999),
        (skill(0), 860),
        (skill(7), 860),
        (skill(8), 1_500),
        (skill(9), 1_500),
        (count(names::INVENTORY, 60), 99),
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
fn the_inverse_of_an_above_cap_edit_needs_storage_only() {
    let gil = gil();
    let mut model = standard_model();
    model.apply(&Edit::set(gil.clone(), Value::Int(20_000_000)), STORAGE_ONLY).unwrap();
    let inverse = model.apply(&Edit::set(gil.clone(), Value::Int(50)), ENFORCE).unwrap();
    assert_eq!(inverse, Edit::set(gil.clone(), Value::Int(20_000_000)));
    let result = model.apply(&inverse, ENFORCE);
    assert!(matches!(result, Err(EditError::AboveCap { cap: 9_999_999, .. })));
    assert_eq!(model.get(&gil).unwrap(), Value::Int(50));
    model.apply(&inverse, STORAGE_ONLY).unwrap();
    assert_eq!(model.get(&gil).unwrap(), Value::Int(20_000_000));
}

#[test]
fn equipment_ids_must_belong_to_the_slot_table_under_enforce() {
    let armor = ids(TableKind::Armor)[0];
    let helmet = ids(TableKind::Helmet)[0];
    let gloves = ids(TableKind::Gloves)[0];
    let weapon = ids(TableKind::Weapon)[0];
    let accepted = [
        (1, 47),
        (2, 47),
        (1, 105),
        (2, weapon),
        (3, armor),
        (3, 209),
        (4, helmet),
        (5, gloves),
        (5, 211),
    ];
    for (number, id) in accepted {
        assert!(try_set(&slot(1, number), i64::from(id), ENFORCE).is_ok(), "slot {number} id {id}");
    }
    // A hand set to a non-weapon, non-shield id, an id of the wrong slot, an unknown id.
    let refused = [(1, armor), (2, helmet), (3, 47), (3, weapon), (4, 209), (5, 210), (1, 999_999), (3, 0)];
    for (number, id) in refused {
        let result = try_set(&slot(1, number), i64::from(id), ENFORCE);
        assert!(
            matches!(result, Err(EditError::UnknownId { id: found, .. }) if found == i64::from(id)),
            "slot {number} id {id}"
        );
    }
    assert!(try_set(&slot(1, 1), 999_999, STORAGE_ONLY).is_ok());
    assert!(try_set(&slot(1, 3), 0, STORAGE_ONLY).is_ok());
}

#[test]
fn the_raw_interface_takes_the_shield_placeholder_in_a_hand() {
    // The Shield table lists 208, so the raw interface holds no rule against it.
    for number in [1, 2] {
        assert!(try_set(&slot(1, number), 208, ENFORCE).is_ok(), "slot {number}");
    }
}

#[test]
fn a_stored_value_outside_the_storage_bound_refuses_the_edit() {
    let mut data = SlotData::standard();
    data.characters[0].hp = -5;
    let text = data.render();
    let mut model = model(&text);
    let result = model.apply(&Edit::set(character_field(1, names::HP), Value::Int(10)), ENFORCE);
    assert!(matches!(result, Err(EditError::OutOfRange { value: -5, .. })));
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn a_stored_non_integer_refuses_the_edit() {
    let path = character_path(0, &[Segment::Doc, Segment::Key("parameter"), Segment::Doc, Segment::Key("currentHP")]);
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    document.set(&path, &Scalar::Float(31.5)).unwrap();
    let text = document.to_json();
    let mut model = Ff2Model::new(document, tables());
    let result = model.apply(&Edit::set(character_field(1, names::HP), Value::Int(10)), ENFORCE);
    assert!(matches!(result, Err(EditError::Format { .. })));
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn a_failed_set_leaves_the_text_untouched() {
    let mut data = SlotData::standard();
    data.spaced = true;
    let text = data.render();
    let mut model = model(&text);
    let hp = character_field(1, names::HP);
    let cases = [
        (FieldId::new("nothing"), Value::Int(1), ENFORCE),
        (character_field(9, names::HP), Value::Int(1), ENFORCE),
        (hp.clone(), Value::Text("x".to_owned()), ENFORCE),
        (hp.clone(), Value::Int(-1), STORAGE_ONLY),
        (hp.clone(), Value::Int(10_000), ENFORCE),
        (character_field(1, names::JOB), Value::Int(2), STORAGE_ONLY),
        (slot(1, 1), Value::Int(209), ENFORCE),
    ];
    for (field, value, policy) in cases {
        assert!(model.apply(&Edit::set(field.clone(), value), policy).is_err(), "{field}");
        assert_eq!(model.document().to_json(), text, "{field}");
    }
}

#[test]
fn a_change_set_applies_all_or_none_through_the_model() {
    let text = SlotData::standard().render();
    let hp = character_field(1, names::HP);
    let bad: ChangeSet = [Edit::set(hp.clone(), Value::Int(50)), Edit::set(gil(), Value::Int(10_000_000))]
        .into_iter()
        .collect();
    let mut model = model(&text);
    let result = bad.apply_with(CapPolicy::Enforce, |edit, policy| model.apply(edit, policy));
    assert!(matches!(result, Err(EditError::AboveCap { cap: 9_999_999, .. })));
    assert_eq!(model.document().to_json(), text);

    let good: ChangeSet = [Edit::set(hp.clone(), Value::Int(50)), Edit::set(gil(), Value::Int(900_000))]
        .into_iter()
        .collect();
    let inverse = good.apply_with(CapPolicy::Enforce, |edit, policy| model.apply(edit, policy)).unwrap();
    assert_eq!(model.get(&hp).unwrap(), Value::Int(50));
    inverse.apply_with(CapPolicy::StorageOnly, |edit, policy| model.apply(edit, policy)).unwrap();
    assert_eq!(model.document().to_json(), text);
}

/// An item id with a row in the Item table and none in the inventory.
fn unowned_item(model: &mut Ff2Model) -> u32 {
    let owned = model.rows(&FieldId::new(names::INVENTORY)).unwrap();
    ids(TableKind::Item).into_iter().find(|id| !owned.contains(id)).unwrap()
}

#[test]
fn the_raw_interface_inserts_and_removes_item_rows() {
    let text = SlotData::standard().render();
    let mut model = model(&text);
    let unowned = unowned_item(&mut model);
    let insert = Edit::insert(row(names::INVENTORY, unowned), None, vec![(names::COUNT, Value::Int(4))]);
    let inverse = model.apply(&insert, ENFORCE).unwrap();
    assert_eq!(inverse, Edit::remove(row(names::INVENTORY, unowned)));
    assert_eq!(model.get(&count(names::INVENTORY, unowned)).unwrap(), Value::Int(4));
    let back = model.apply(&inverse, ENFORCE).unwrap();
    assert_eq!(model.document().to_json(), text);
    // The inverse of the removal inserts the row where it stood.
    assert!(matches!(back, Edit::Insert { position: Some(_), .. }));

    // No game rule: a placeholder row and a key item with another count pass.
    let placeholder = Edit::remove(row(names::INVENTORY, 47));
    let inverse = model.apply(&placeholder, ENFORCE).unwrap();
    model.apply(&inverse, STORAGE_ONLY).unwrap();
    assert_eq!(model.document().to_json(), text);
    let key = key_items()[0];
    model.apply(&Edit::set(count(names::KEY_ITEMS, key), Value::Int(3)), ENFORCE).unwrap();
    assert_eq!(model.get(&count(names::KEY_ITEMS, key)).unwrap(), Value::Int(3));
}

#[test]
fn insert_and_remove_refuse_what_they_cannot_restore() {
    let text = SlotData::standard().render();
    let mut model = model(&text);
    let character = row(names::CHARACTER, 1);
    assert!(model.apply(&Edit::remove(character.clone()), ENFORCE).is_err());
    assert!(model.apply(&Edit::insert(row(names::CHARACTER, 9), None, Vec::new()), ENFORCE).is_err());
    let duplicate = Edit::insert(row(names::INVENTORY, 60), None, Vec::new());
    assert!(matches!(model.apply(&duplicate, ENFORCE), Err(EditError::DuplicateRow { .. })));
    let unknown = Edit::insert(row(names::INVENTORY, 999_999), None, Vec::new());
    assert!(matches!(model.apply(&unknown, ENFORCE), Err(EditError::UnknownId { .. })));
    let far = Edit::insert(row(names::INVENTORY, unowned_item(&mut model)), Some(500), Vec::new());
    assert!(matches!(model.apply(&far, ENFORCE), Err(EditError::PositionOutOfRange { .. })));
    assert_eq!(model.document().to_json(), text);
}
