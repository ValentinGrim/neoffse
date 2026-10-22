//! `apply_game`: the equip plan, the pass-through edits, undo and redo
//! through the raw interface. Every equip case ends with the game rules
//! checked on the whole slot.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_rules::assert_game_rules;
use common::ff1_save::{Row, SlotData, character_field, model, standard_model};
use neoffse_core::{CapPolicy, ChangeSet, Edit, FieldId, RowId, Value};
use neoffse_format_pr::ff1::{Ff1Model, GameChange, names};

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

fn row(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

fn equip(id: u32, slot_number: u32, content: i64) -> Edit {
    Edit::set(slot(id, slot_number), Value::Int(content))
}

fn int(model: &mut Ff1Model, field: &FieldId) -> i64 {
    match model.get(field).unwrap() {
        Value::Int(value) => value,
        other => panic!("{field}: {other:?}"),
    }
}

/// Every inventory row with its count, in document order.
fn inventory(model: &mut Ff1Model) -> Vec<(u32, i64)> {
    let list = FieldId::new(names::INVENTORY);
    model
        .rows(&list)
        .unwrap()
        .into_iter()
        .map(|key| (key, int(model, &count(names::INVENTORY, key))))
        .collect()
}

/// Every slot `contentId` and `count` of the four characters.
fn slots(model: &mut Ff1Model) -> Vec<(u32, u32, i64, i64)> {
    let mut found = Vec::new();
    for id in 1..=4 {
        for number in 1..=5 {
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

fn bare_slot() -> SlotData {
    let mut data = SlotData::standard();
    "0".clone_into(&mut data.characters[2].equipment[2].content_id);
    data.characters[2].equipment[2].count = Some("0".to_owned());
    data
}

fn plain(change: &GameChange) -> (Vec<Edit>, Vec<Edit>) {
    (change.forward.edits().to_vec(), change.inverse.edits().to_vec())
}

fn is_set(edit: &Edit) -> bool {
    matches!(edit, Edit::Set { .. })
}

#[test]
fn equip_an_owned_unworn_item() {
    let mut model = standard_model();
    let before = inventory(&mut model);
    let other = slots(&mut model);
    let change = model.apply_game(&equip(1, 1, 100)).unwrap();
    let want = [equip(1, 1, 100), Edit::set(slot_count(1, 1), Value::Int(1))];
    assert_eq!(change.forward.edits(), want);
    assert_eq!(int(&mut model, &slot(1, 1)), 100);
    assert_eq!(int(&mut model, &slot_count(1, 1)), 1);
    assert_eq!(inventory(&mut model), before);
    let after = slots(&mut model);
    for (old, new) in other.iter().zip(&after) {
        if (old.0, old.1) != (1, 1) {
            assert_eq!(old, new);
        }
    }
    assert_game_rules(&mut model, true);
}

#[test]
fn equip_an_unowned_item_inserts_its_row() {
    let mut model = standard_model();
    let mut want_rows = model.rows(&FieldId::new(names::INVENTORY)).unwrap();
    let change = model.apply_game(&equip(2, 1, 104)).unwrap();
    let want = [
        equip(2, 1, 104),
        Edit::insert(row(104), None, vec![(names::COUNT, Value::Int(1))]),
        Edit::set(slot_count(2, 1), Value::Int(1)),
    ];
    assert_eq!(change.forward.edits(), want);
    want_rows.push(104);
    assert_eq!(model.rows(&FieldId::new(names::INVENTORY)).unwrap(), want_rows);
    assert_eq!(int(&mut model, &slot_count(2, 1)), 1);
    assert_game_rules(&mut model, true);
}

#[test]
fn equip_an_item_worn_by_others_raises_the_count() {
    let mut model = standard_model();
    let change = model.apply_game(&equip(1, 1, 92)).unwrap();
    assert_eq!(change.forward.len(), 3);
    assert_eq!(change.forward.edits()[1], Edit::set(count(names::INVENTORY, 92), Value::Int(2)));
    assert_eq!(int(&mut model, &count(names::INVENTORY, 92)), 2);
    assert_eq!(int(&mut model, &slot_count(1, 1)), 2);
    assert_eq!(int(&mut model, &slot_count(2, 1)), 1, "other wearers keep their copied count");
    model.apply_game(&equip(2, 4, 183)).unwrap();
    assert_eq!(int(&mut model, &count(names::INVENTORY, 183)), 3);
    assert_eq!(int(&mut model, &slot_count(2, 4)), 3);
    assert_game_rules(&mut model, false);
}

#[test]
fn equip_an_item_already_counted_enough_adds_no_row_edit() {
    let mut data = SlotData::standard();
    with_count(&mut data, 100, 5);
    let mut model = model(&data.render());
    let change = model.apply_game(&equip(1, 1, 100)).unwrap();
    let want = [equip(1, 1, 100), Edit::set(slot_count(1, 1), Value::Int(5))];
    assert_eq!(change.forward.edits(), want);
    assert_eq!(int(&mut model, &count(names::INVENTORY, 100)), 5);
}

#[test]
fn equip_the_same_item_writes_the_stored_values() {
    let text = SlotData::standard().render();
    let mut model = model(&text);
    let change = model.apply_game(&equip(1, 1, 89)).unwrap();
    assert_eq!(change.forward.len(), 2);
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn unequip_writes_the_placeholder_and_its_count() {
    let mut standard = standard_model();
    let before = inventory(&mut standard);
    let change = standard.apply_game(&equip(1, 1, 62)).unwrap();
    let want = [equip(1, 1, 62), Edit::set(slot_count(1, 1), Value::Int(5))];
    assert_eq!(change.forward.edits(), want);
    assert_eq!(int(&mut standard, &slot(1, 1)), 62);
    assert_eq!(int(&mut standard, &slot_count(1, 1)), 5);
    assert_eq!(inventory(&mut standard), before, "the item that left stays, the placeholder row is untouched");

    let mut data = SlotData::standard();
    without_placeholders(&mut data);
    let mut bare = model(&data.render());
    let change = bare.apply_game(&equip(1, 1, 62)).unwrap();
    assert_eq!(change.forward.edits(), [equip(1, 1, 62)]);
    assert_eq!(int(&mut bare, &slot_count(1, 1)), 1, "the slot count stays as read");
}

/// The equip cases above, as a starting text and the edit.
fn cases() -> Vec<(String, Edit)> {
    let standard = SlotData::standard().render();
    let mut enough = SlotData::standard();
    with_count(&mut enough, 100, 5);
    let mut bare = SlotData::standard();
    without_placeholders(&mut bare);
    vec![
        (standard.clone(), equip(1, 1, 100)),
        (standard.clone(), equip(2, 1, 104)),
        (standard.clone(), equip(1, 1, 92)),
        (standard.clone(), equip(2, 4, 183)),
        (enough.render(), equip(1, 1, 100)),
        (standard.clone(), equip(1, 1, 89)),
        (standard, equip(1, 1, 62)),
        (bare.render(), equip(1, 1, 62)),
        (bare_slot().render(), equip(3, 3, 154)),
    ]
}

fn run(model: &mut Ff1Model, set: &ChangeSet, policy: CapPolicy) -> ChangeSet {
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

#[test]
fn every_edit_is_plain() {
    for (index, (text, edit)) in cases().into_iter().enumerate() {
        let mut model = model(&text);
        let change = model.apply_game(&edit).unwrap();
        let (forward, inverse) = plain(&change);
        assert!(
            forward.iter().all(|edit| is_set(edit) || matches!(edit, Edit::Insert { .. })),
            "forward {index}"
        );
        assert!(
            inverse.iter().all(|edit| is_set(edit) || matches!(edit, Edit::Remove { .. })),
            "inverse {index}"
        );
        let mut again = self::model(&text);
        for step in &forward {
            again.apply(step, ENFORCE).unwrap();
        }
        assert_eq!(again.document().to_json(), model.document().to_json(), "case {index}");
    }
}

#[test]
fn a_bare_slot_holding_zero_can_be_equipped_and_undone() {
    let text = bare_slot().render();
    let mut model = model(&text);
    let change = model.apply_game(&equip(3, 3, 154)).unwrap();
    assert_eq!(int(&mut model, &slot(3, 3)), 154);
    assert_eq!(change.inverse.edits().last().unwrap(), &Edit::set(slot(3, 3), Value::Int(0)));
    run(&mut model, &change.inverse, STORAGE_ONLY);
    assert_eq!(model.document().to_json(), text);
    assert_eq!(int(&mut model, &slot_count(3, 3)), 0);
}

#[test]
fn an_equip_is_atomic() {
    let mut data = SlotData::standard();
    data.spaced = true;
    data.characters[0].equipment[0].count = None;
    let text = data.render();
    let mut model = model(&text);
    let error = model.apply_game(&equip(1, 1, 92)).unwrap_err();
    assert!(matches!(error, neoffse_core::EditError::Format { .. }), "{error}");
    assert_eq!(model.document().to_json(), text);
    assert_eq!(int(&mut model, &count(names::INVENTORY, 92)), 1, "the raised count went back");
    assert_eq!(int(&mut model, &slot(1, 1)), 89);
}

#[test]
fn pass_through_edits_become_one_plain_edit() {
    let edits = [
        Edit::set(character_field(1, names::POWER), Value::Int(40)),
        Edit::insert(row(7), None, Vec::new()),
        Edit::remove(row(100)),
        Edit::remove(RowId::new(FieldId::new(names::KEY_ITEMS), 45)),
        Edit::insert(RowId::new(FieldId::new(names::KEY_ITEMS), 47), None, Vec::new()),
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
fn row_locked_agrees_with_the_game_remove() {
    let mut model = standard_model();
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        for key in model.rows(&FieldId::new(list)).unwrap() {
            let target = RowId::new(FieldId::new(list), key);
            let locked = model.row_locked(&target).unwrap();
            let mut probe = model.clone();
            let refused = matches!(probe.apply_game(&Edit::remove(target)), Err(neoffse_core::EditError::ReadOnly { .. }));
            assert_eq!(locked, refused, "{list}.{key}");
        }
    }
}
