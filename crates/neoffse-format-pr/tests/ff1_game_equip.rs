//! More `apply_game` cases: the stat caps, an equip with a second wearer,
//! the placeholder lock before the value check, unequip on the left hand and
//! the answers on an absent row.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{SlotData, character_field, model, standard_model};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, RowId, TableKind, Value};
use neoffse_format_pr::ff1::{Ff1Model, GAME, names};

fn slot(id: u32, number: u32) -> FieldId {
    character_field(id, names::EQUIPMENT).with_id(number)
}

fn slot_count(id: u32, number: u32) -> FieldId {
    slot(id, number).with_name(names::COUNT)
}

fn row(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

fn count(key: u32) -> FieldId {
    row(key).field(names::COUNT)
}

fn int(model: &mut Ff1Model, field: &FieldId) -> i64 {
    match model.get(field).unwrap() {
        Value::Int(value) => value,
        other => panic!("{field}: {other:?}"),
    }
}

fn placeholder(kind: TableKind) -> u32 {
    neoffse_data::tables_for(GAME).unwrap().placeholder(kind).unwrap().id
}

#[test]
fn a_stat_above_its_game_cap_is_refused_unless_the_policy_is_storage_only() {
    let power = character_field(1, names::POWER);
    let edit = Edit::set(power.clone(), Value::Int(100));
    let raw = standard_model().apply(&edit, CapPolicy::Enforce);
    assert!(matches!(raw, Err(EditError::AboveCap { cap: 99, value: 100, .. })));
    let game = standard_model().apply_game(&edit);
    assert!(matches!(game, Err(EditError::AboveCap { cap: 99, value: 100, .. })));
    let mut model = standard_model();
    model.apply(&edit, CapPolicy::StorageOnly).unwrap();
    assert_eq!(int(&mut model, &power), 100);
    model.apply(&Edit::set(power, Value::Int(99)), CapPolicy::Enforce).unwrap();
}

#[test]
fn a_second_wearer_raises_the_inserted_count_to_two() {
    let mut data = SlotData::standard();
    data.characters[1].equipment[0].content_id = "104".to_owned();
    let mut model = model(&data.render());
    assert!(model.get(&count(104)).is_err(), "the row is absent");
    let change = model.apply_game(&Edit::set(slot(1, 1), Value::Int(104))).unwrap();
    let want = [
        Edit::set(slot(1, 1), Value::Int(104)),
        Edit::insert(row(104), None, vec![(names::COUNT, Value::Int(2))]),
        Edit::set(slot_count(1, 1), Value::Int(2)),
    ];
    assert_eq!(change.forward.edits(), want);
    assert_eq!(int(&mut model, &count(104)), 2);
}

#[test]
fn the_placeholder_lock_fires_before_the_value_check_on_insert() {
    let weapon = placeholder(TableKind::Weapon);
    let mut model = standard_model();
    let text = model.document().to_json();
    let insert = Edit::insert(row(weapon), None, vec![(names::COUNT, Value::Int(0))]);
    let error = model.apply_game(&insert).unwrap_err();
    assert!(matches!(error, EditError::ReadOnly { ref field } if field == row(weapon).id()), "{error}");
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn unequip_on_the_left_hand_copies_the_shield_placeholder_row() {
    let shield_placeholder = placeholder(TableKind::Shield);
    let mut model = standard_model();
    // Character 4 wears shield 166; give it to character 1 first.
    model.apply_game(&Edit::set(slot(1, 2), Value::Int(166))).unwrap();
    let shield_count = int(&mut model, &count(166));
    let placeholder_count = int(&mut model, &count(shield_placeholder));
    let change = model.apply_game(&Edit::set(slot(1, 2), Value::Int(i64::from(shield_placeholder)))).unwrap();
    let want = [
        Edit::set(slot(1, 2), Value::Int(i64::from(shield_placeholder))),
        Edit::set(slot_count(1, 2), Value::Int(placeholder_count)),
    ];
    assert_eq!(change.forward.edits(), want);
    assert_eq!(int(&mut model, &slot(1, 2)), i64::from(shield_placeholder));
    assert_eq!(int(&mut model, &slot_count(1, 2)), placeholder_count);
    assert_eq!(int(&mut model, &count(166)), shield_count, "the shield stays in the inventory");
}

#[test]
fn an_absent_row_is_missing_on_both_interfaces() {
    let mut model = standard_model();
    let absent = row(999);
    assert!(matches!(model.row_locked(&absent), Err(EditError::MissingRow { row }) if row == *absent.id()));
    let result = model.apply_game(&Edit::remove(absent.clone()));
    assert!(matches!(result, Err(EditError::MissingRow { row }) if row == *absent.id()));
    let key_item = RowId::new(FieldId::new(names::KEY_ITEMS), 999);
    assert!(matches!(model.row_locked(&key_item), Err(EditError::MissingRow { .. })));
    assert!(matches!(model.apply_game(&Edit::remove(key_item)), Err(EditError::MissingRow { .. })));
}

#[test]
fn an_absent_placeholder_row_is_missing_for_row_locked_and_read_only_for_apply_game() {
    let weapon = placeholder(TableKind::Weapon);
    let mut data = SlotData::standard();
    data.inventory.drain(..5);
    let mut model = model(&data.render());
    let target = row(weapon);
    assert!(matches!(model.row_locked(&target), Err(EditError::MissingRow { .. })));
    let result = model.apply_game(&Edit::remove(target.clone()));
    assert!(matches!(result, Err(EditError::ReadOnly { field }) if field == *target.id()));
}
