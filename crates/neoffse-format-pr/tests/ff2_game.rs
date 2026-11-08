//! FF II `apply_game`: the equip plan with two hands, the unequip to the empty
//! hand 47, the pass-through edits, undo and redo through the raw interface.
//! Every equip case ends with the game rules checked on the whole slot.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff2_rules::assert_game_rules;
use common::ff2_save::{Row, SlotData, character_field, ids, key_items, model, standard_model};
use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FieldId, GameChange, RowId, TableKind, Value};
use neoffse_format_pr::ff2::{Ff2Model, names};

const ENFORCE: CapPolicy = CapPolicy::Enforce;
const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;

fn slot(id: u32, slot: u32) -> FieldId {
    character_field(id, names::EQUIPMENT).with_id(slot)
}

fn slot_count(id: u32, slot_number: u32) -> FieldId {
    slot(id, slot_number).with_name(names::COUNT)
}

fn count(list: &'static str, key: u32) -> FieldId {
    FieldId::new(list).with_id(key).with_name(names::COUNT)
}

fn row(list: &'static str, key: u32) -> RowId {
    RowId::new(FieldId::new(list), key)
}

fn equip(id: u32, slot_number: u32, content: u32) -> Edit {
    Edit::set(slot(id, slot_number), Value::Int(i64::from(content)))
}

fn int(model: &mut Ff2Model, field: &FieldId) -> i64 {
    match model.get(field).unwrap() {
        Value::Int(value) => value,
        other => panic!("{field}: {other:?}"),
    }
}

/// Every inventory row with its count, in document order.
fn inventory(model: &mut Ff2Model) -> Vec<(u32, i64)> {
    let list = FieldId::new(names::INVENTORY);
    model
        .rows(&list)
        .unwrap()
        .into_iter()
        .map(|key| (key, int(model, &count(names::INVENTORY, key))))
        .collect()
}

/// Every slot `contentId` and `count` of the four characters.
fn slots(model: &mut Ff2Model) -> Vec<(u32, u32, i64, i64)> {
    let mut found = Vec::new();
    for id in 1..=4 {
        for number in 1..=names::EQUIPMENT_SLOTS {
            found.push((id, number, int(model, &slot(id, number)), int(model, &slot_count(id, number))));
        }
    }
    found
}

fn with_count(data: &mut SlotData, key: u32, new_count: u32) {
    let text = format!("{{\"contentId\":{key},\"count\":");
    let index = data.inventory.iter().position(|row| row.text.starts_with(&text)).unwrap();
    data.inventory[index] = Row::new(key, new_count);
}

fn without_placeholders(data: &mut SlotData) {
    data.inventory.drain(..5);
}

/// A weapon with a row in the Weapon table and none in the standard inventory.
fn unowned_weapon() -> u32 {
    let mut model = standard_model();
    let rows = model.rows(&FieldId::new(names::INVENTORY)).unwrap();
    ids(TableKind::Weapon).into_iter().find(|id| !rows.contains(id)).unwrap()
}

fn unworn_weapon() -> u32 {
    ids(TableKind::Weapon)[10]
}

#[test]
fn equip_an_owned_unworn_item() {
    let mut model = standard_model();
    let before = inventory(&mut model);
    let other = slots(&mut model);
    let weapon = unworn_weapon();
    let change = model.apply_game(&equip(1, 1, weapon)).unwrap();
    let want = [equip(1, 1, weapon), Edit::set(slot_count(1, 1), Value::Int(1))];
    assert_eq!(change.forward.edits(), want);
    assert_eq!(int(&mut model, &slot(1, 1)), i64::from(weapon));
    assert_eq!(int(&mut model, &slot_count(1, 1)), 1);
    assert_eq!(inventory(&mut model), before);
    for (old, new) in other.iter().zip(&slots(&mut model)) {
        if (old.0, old.1) != (1, 1) {
            assert_eq!(old, new);
        }
    }
    assert_game_rules(&mut model, true);
}

#[test]
fn equip_an_unowned_item_inserts_its_row() {
    let mut model = standard_model();
    let weapon = unowned_weapon();
    let mut want_rows = model.rows(&FieldId::new(names::INVENTORY)).unwrap();
    let change = model.apply_game(&equip(2, 2, weapon)).unwrap();
    let want = [
        equip(2, 2, weapon),
        Edit::insert(row(names::INVENTORY, weapon), None, vec![(names::COUNT, Value::Int(1))]),
        Edit::set(slot_count(2, 2), Value::Int(1)),
    ];
    assert_eq!(change.forward.edits(), want);
    want_rows.push(weapon);
    assert_eq!(model.rows(&FieldId::new(names::INVENTORY)).unwrap(), want_rows);
    assert_eq!(int(&mut model, &slot_count(2, 2)), 1);
    assert_game_rules(&mut model, true);
}

#[test]
fn the_same_item_in_both_hands_raises_the_row_to_two() {
    // Character 2 holds 91 in hand 1 and has one in the inventory.
    let mut model = standard_model();
    let change = model.apply_game(&equip(2, 2, 91)).unwrap();
    let want = [
        equip(2, 2, 91),
        Edit::set(count(names::INVENTORY, 91), Value::Int(2)),
        Edit::set(slot_count(2, 2), Value::Int(2)),
    ];
    assert_eq!(change.forward.edits(), want);
    assert_eq!(int(&mut model, &count(names::INVENTORY, 91)), 2);
    assert_eq!(int(&mut model, &slot_count(2, 2)), 2);
    assert_eq!(int(&mut model, &slot_count(2, 1)), 1, "the other hand keeps its copied count");
    assert_game_rules(&mut model, false);
}

#[test]
fn the_wearer_count_counts_slots_not_characters() {
    // Character 1 holds 60 in both hands: two wearers, one character.
    let mut data = SlotData::standard();
    with_count(&mut data, 60, 2);
    let mut model = model(&data.render());
    let change = model.apply_game(&equip(3, 2, 60)).unwrap();
    let want = [
        equip(3, 2, 60),
        Edit::set(count(names::INVENTORY, 60), Value::Int(3)),
        Edit::set(slot_count(3, 2), Value::Int(3)),
    ];
    assert_eq!(change.forward.edits(), want);
    assert_game_rules(&mut model, false);
}

#[test]
fn equip_an_item_already_counted_enough_adds_no_row_edit() {
    let mut model = standard_model();
    // 60 is worn twice and counted 3: a third wearer needs no raise.
    let change = model.apply_game(&equip(3, 2, 60)).unwrap();
    let want = [equip(3, 2, 60), Edit::set(slot_count(3, 2), Value::Int(3))];
    assert_eq!(change.forward.edits(), want);
    assert_eq!(int(&mut model, &count(names::INVENTORY, 60)), 3);
}

#[test]
fn equip_the_same_item_writes_the_stored_values() {
    let text = SlotData::standard().render();
    let mut model = model(&text);
    let change = model.apply_game(&equip(1, 1, 60)).unwrap();
    assert_eq!(change.forward.len(), 2);
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn unequip_writes_the_empty_hand_and_its_count_in_each_hand() {
    // 47 is the empty hand in both hands, with the count of its row.
    for (id, hand) in [(1, 1), (1, 2), (2, 1), (2, 2), (4, 2)] {
        let mut standard = standard_model();
        let before = inventory(&mut standard);
        let change = standard.apply_game(&equip(id, hand, 47)).unwrap();
        let want = [equip(id, hand, 47), Edit::set(slot_count(id, hand), Value::Int(10))];
        assert_eq!(change.forward.edits(), want, "character {id} hand {hand}");
        assert_eq!(int(&mut standard, &slot(id, hand)), 47);
        assert_eq!(inventory(&mut standard), before, "the item that left stays, the placeholder row is untouched");
        assert_game_rules(&mut standard, false);
    }
}

#[test]
fn unequip_an_armor_piece_writes_its_placeholder() {
    let mut model = standard_model();
    let change = model.apply_game(&equip(1, 3, 209)).unwrap();
    let want = [equip(1, 3, 209), Edit::set(slot_count(1, 3), Value::Int(5))];
    assert_eq!(change.forward.edits(), want);
}

#[test]
fn unequip_without_the_placeholder_row_leaves_the_count() {
    let mut data = SlotData::standard();
    without_placeholders(&mut data);
    let mut bare = model(&data.render());
    let change = bare.apply_game(&equip(1, 1, 47)).unwrap();
    assert_eq!(change.forward.edits(), [equip(1, 1, 47)]);
    assert_eq!(int(&mut bare, &slot_count(1, 1)), 3, "the slot count stays as read");
}

/// The equip cases above, as a starting text and the edit.
fn cases() -> Vec<(String, Edit)> {
    let standard = SlotData::standard().render();
    let mut two = SlotData::standard();
    with_count(&mut two, 60, 2);
    let mut bare = SlotData::standard();
    without_placeholders(&mut bare);
    vec![
        (standard.clone(), equip(1, 1, unworn_weapon())),
        (standard.clone(), equip(2, 2, unowned_weapon())),
        (standard.clone(), equip(2, 2, 91)),
        (two.render(), equip(3, 2, 60)),
        (standard.clone(), equip(3, 2, 60)),
        (standard.clone(), equip(1, 1, 60)),
        (standard.clone(), equip(1, 1, 47)),
        (standard.clone(), equip(1, 2, 47)),
        (standard.clone(), equip(4, 2, 47)),
        (standard, equip(1, 3, 209)),
        (bare.render(), equip(1, 1, 47)),
    ]
}

fn run(model: &mut Ff2Model, set: &ChangeSet, policy: CapPolicy) -> ChangeSet {
    set.apply_with(policy, |edit, policy| model.apply(edit, policy)).unwrap()
}

#[test]
fn undo_and_redo_restore_the_bytes() {
    for (index, (text, edit)) in cases().into_iter().enumerate() {
        let mut model = model(&text);
        let change = model.apply_game(&edit).unwrap();
        let applied = model.document().to_json();
        run(&mut model, &change.inverse, STORAGE_ONLY);
        assert_eq!(model.document().to_json(), text, "undo of case {index}");
        run(&mut model, &change.forward, STORAGE_ONLY);
        assert_eq!(model.document().to_json(), applied, "redo of case {index}");
    }
}

fn plain(change: &GameChange) -> (&[Edit], &[Edit]) {
    (change.forward.edits(), change.inverse.edits())
}

#[test]
fn every_edit_is_plain() {
    for (index, (text, edit)) in cases().into_iter().enumerate() {
        let mut model = model(&text);
        let change = model.apply_game(&edit).unwrap();
        let (forward, inverse) = plain(&change);
        assert!(
            forward.iter().all(|edit| matches!(edit, Edit::Set { .. } | Edit::Insert { .. })),
            "forward {index}"
        );
        assert!(
            inverse.iter().all(|edit| matches!(edit, Edit::Set { .. } | Edit::Remove { .. })),
            "inverse {index}"
        );
        let mut again = self::model(&text);
        for step in forward {
            again.apply(step, ENFORCE).unwrap();
        }
        assert_eq!(again.document().to_json(), model.document().to_json(), "case {index}");
    }
}

#[test]
fn an_equip_is_atomic() {
    let mut data = SlotData::standard();
    data.spaced = true;
    data.characters[1].equipment[1].count = None;
    let text = data.render();
    let mut model = model(&text);
    // The slot and the raised row write, then the slot count has no key to set.
    let error = model.apply_game(&equip(2, 2, 91)).unwrap_err();
    assert!(matches!(error, EditError::Format { .. }), "{error}");
    assert_eq!(model.document().to_json(), text);
    assert_eq!(int(&mut model, &count(names::INVENTORY, 91)), 1, "the raised count went back");
    assert_eq!(int(&mut model, &slot(2, 2)), 105);
}

#[test]
fn pass_through_edits_become_one_plain_edit() {
    let key_item = ids(TableKind::KeyItem).into_iter().find(|id| !key_items().contains(id)).unwrap();
    let edits = [
        Edit::set(character_field(1, names::POWER), Value::Int(40)),
        Edit::set(character_field(1, names::HP), Value::Int(77)),
        Edit::set(character_field(1, names::SKILL).with_id(2), Value::Int(300)),
        Edit::set(FieldId::new(names::GIL), Value::Int(500)),
        Edit::insert(row(names::INVENTORY, ids(TableKind::Item)[3]), None, Vec::new()),
        Edit::remove(row(names::INVENTORY, unworn_weapon())),
        Edit::remove(row(names::KEY_ITEMS, key_items()[0])),
        Edit::insert(row(names::KEY_ITEMS, key_item), None, Vec::new()),
    ];
    for edit in edits {
        let mut game = standard_model();
        let mut raw = standard_model();
        let change = game.apply_game(&edit).unwrap();
        let inverse = raw.apply(&edit, ENFORCE).unwrap();
        assert_eq!(change.forward.edits(), std::slice::from_ref(&edit), "{}", edit.target());
        assert_eq!(change.inverse.edits(), [inverse], "{}", edit.target());
        assert_eq!(game.document().to_json(), raw.document().to_json());
    }
}

#[test]
fn a_computed_or_slot_count_set_is_refused_before_any_write() {
    let text = SlotData::standard().render();
    let mut model = model(&text);
    for field in [slot_count(1, 1), character_field(1, names::MAX_HP), character_field(1, names::JOB)] {
        let result = model.apply_game(&Edit::set(field.clone(), Value::Int(3)));
        assert!(matches!(result, Err(EditError::ReadOnly { field: named }) if named == field), "{field}");
    }
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn a_set_below_the_wearers_is_refused_and_at_them_accepted() {
    let mut model = standard_model();
    let sixty = count(names::INVENTORY, 60);
    let result = model.apply_game(&Edit::set(sixty.clone(), Value::Int(1)));
    assert!(matches!(result, Err(EditError::OutOfRange { min: 2, .. })));
    assert!(model.apply_game(&Edit::set(sixty, Value::Int(2))).is_ok());
}
