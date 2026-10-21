//! `field_locked`: which fields the game interface refuses to set whatever
//! the value. It reads the tables and writes nothing.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{SlotData, character_field, model, standard_model};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, Value};
use neoffse_format_pr::ff1::names;

fn count(list: &'static str, key: u32) -> FieldId {
    FieldId::new(list).with_id(key).with_name(names::COUNT)
}

#[test]
fn slot_counts_are_locked() {
    let model = standard_model();
    for id in 1..=4 {
        for number in 1..=5 {
            let field = character_field(id, names::EQUIPMENT).with_id(number).with_name(names::COUNT);
            assert!(model.field_locked(&field).unwrap(), "{field}");
        }
    }
}

#[test]
fn placeholder_row_counts_are_locked() {
    let present = standard_model();
    for key in [62, 130, 131, 132, 133] {
        assert!(present.field_locked(&count(names::INVENTORY, key)).unwrap(), "inventory.{key}");
    }
    let mut data = SlotData::standard();
    data.inventory.drain(..5);
    let absent = model(&data.render());
    for key in [62, 130, 131, 132, 133] {
        assert!(absent.field_locked(&count(names::INVENTORY, key)).unwrap(), "absent inventory.{key}");
    }
}

#[test]
fn key_item_counts_are_locked() {
    let mut model = standard_model();
    for key in model.rows(&FieldId::new(names::KEY_ITEMS)).unwrap() {
        assert!(model.field_locked(&count(names::KEY_ITEMS, key)).unwrap(), "key_items.{key}");
    }
    // The lock needs no row: a key item the slot lacks is locked too.
    assert!(model.field_locked(&count(names::KEY_ITEMS, 47)).unwrap());
}

#[test]
fn every_other_field_is_unlocked() {
    let model = standard_model();
    let others = names::character_fields(1)
        .into_iter()
        .filter(|field| field.keys().last() != Some(&neoffse_core::Key::Name(names::COUNT)));
    let mut seen = 0;
    for field in others {
        assert!(!model.field_locked(&field).unwrap(), "{field}");
        seen += 1;
    }
    assert_eq!(seen, 34);
    for field in [FieldId::new(names::GIL), count(names::INVENTORY, 89), count(names::INVENTORY, 202)] {
        assert!(!model.field_locked(&field).unwrap(), "{field}");
    }
}

#[test]
fn unknown_paths_are_unknown_fields() {
    let model = standard_model();
    for field in [
        character_field(1, "nope"),
        FieldId::new(names::INVENTORY).with_id(89).with_name("name"),
        FieldId::new("nothing"),
    ] {
        assert!(matches!(model.field_locked(&field), Err(EditError::UnknownField { .. })), "{field}");
    }
}

#[test]
fn field_locked_agrees_with_the_game_set() {
    let mut model = standard_model();
    let mut fields = Vec::new();
    for id in [1, 2] {
        fields.extend(names::character_fields(id));
    }
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        for key in model.rows(&FieldId::new(list)).unwrap() {
            fields.push(count(list, key));
        }
    }
    let mut key_item_counts = 0;
    for field in fields {
        let value = model.get(&field).unwrap();
        let edit = Edit::set(field.clone(), value);
        let mut raw = model.clone();
        assert!(raw.apply(&edit, CapPolicy::Enforce).is_ok(), "raw {field}");
        let locked = model.field_locked(&field).unwrap();
        if field.to_string().starts_with(names::KEY_ITEMS) {
            assert!(locked, "{field}");
            key_item_counts += 1;
        }
        let mut game = model.clone();
        match game.apply_game(&edit) {
            Err(EditError::ReadOnly { .. }) => assert!(locked, "{field}"),
            Ok(_) => assert!(!locked, "{field}"),
            Err(other) => panic!("{field}: {other}"),
        }
    }
    assert_eq!(key_item_counts, 9);
}

#[test]
fn the_wearers_floor_does_not_lock_the_field() {
    let mut model = standard_model();
    let field = count(names::INVENTORY, 183);
    assert!(!model.field_locked(&field).unwrap());
    let error = model.apply_game(&Edit::set(field, Value::Int(1))).unwrap_err();
    assert!(matches!(error, EditError::OutOfRange { min: 2, .. }), "{error}");
}

#[test]
fn field_locked_changes_nothing() {
    let mut data = SlotData::standard();
    data.spaced = true;
    let text = data.render();
    let spaced = model(&text);
    for field in names::character_fields(1)
        .into_iter()
        .chain([count(names::INVENTORY, 62), FieldId::new("nothing")])
    {
        let _ = spaced.field_locked(&field);
        assert_eq!(spaced.document().to_json(), text, "{field}");
    }
}
