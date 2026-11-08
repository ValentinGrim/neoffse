//! `field_locked` and `row_locked` against what `apply_game` refuses, and the
//! key-item rule: a key item keeps a `count` of 1
//! (`docs/formats/pixel-remaster.md#inventory`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_field;
use common::ff2_real;
use common::ff2_save::{SlotData, ids, key_items, standard_model};
use common::{count, row};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, TableKind, Value};
use neoffse_format_pr::ff2::{Ff2Model, names};

const PLACEHOLDERS: [u32; 5] = [47, 208, 209, 210, 211];

fn locked(field: &FieldId) -> bool {
    standard_model().field_locked(field).unwrap()
}

#[test]
fn the_locked_fields_are_the_slot_counts_the_computed_ones_and_the_person() {
    for number in 1..=names::EQUIPMENT_SLOTS {
        let slot = character_field(1, names::EQUIPMENT).with_id(number);
        assert!(locked(&slot.clone().with_name(names::COUNT)), "slot {number} count");
        assert!(!locked(&slot), "slot {number}");
    }
    for name in [names::MAX_HP, names::MAX_MP, names::JOB] {
        assert!(locked(&character_field(1, name)), "{name}");
    }
    let open = [
        names::NAME,
        names::HP,
        names::MP,
        names::MAX_HP_GAIN,
        names::MAX_MP_GAIN,
        names::POWER,
        names::VITALITY,
        names::AGILITY,
        names::INTELLIGENCE,
        names::SPIRIT,
        names::MAGIC,
    ];
    for name in open {
        assert!(!locked(&character_field(1, name)), "{name}");
    }
    for key in 0..names::SKILLS {
        assert!(!locked(&character_field(1, names::SKILL).with_id(key)), "skill {key}");
    }
    assert!(!locked(&FieldId::new(names::GIL)));
}

#[test]
fn the_counts_of_the_placeholder_rows_and_of_every_key_item_are_locked() {
    for key in PLACEHOLDERS {
        assert!(locked(&count(names::INVENTORY, key)), "placeholder {key}");
    }
    // A key item that the slot does not hold is locked too: the lock reads no document.
    for key in ids(TableKind::KeyItem) {
        assert!(locked(&count(names::KEY_ITEMS, key)), "key item {key}");
    }
    assert!(!locked(&count(names::INVENTORY, 60)));
    assert!(!locked(&count(names::INVENTORY, ids(TableKind::Item)[0])));
}

#[test]
fn an_undeclared_field_is_unknown() {
    let model = standard_model();
    for field in [character_field(1, "level"), character_field(1, "exp"), FieldId::new("nothing")] {
        assert!(matches!(model.field_locked(&field), Err(EditError::UnknownField { .. })), "{field}");
    }
}

/// Every field the lock speaks about, on the standard slot.
fn fields(model: &mut Ff2Model) -> Vec<FieldId> {
    let mut all = vec![FieldId::new(names::GIL)];
    for id in model.rows(&FieldId::new(names::CHARACTER)).unwrap() {
        all.extend(names::character_fields(id));
        all.extend(names::derived_fields(id));
    }
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        for key in model.rows(&FieldId::new(list)).unwrap() {
            all.push(count(list, key));
        }
    }
    all
}

#[test]
fn field_locked_agrees_with_apply_game() {
    let mut model = standard_model();
    for field in fields(&mut model) {
        let value = model.get(&field).unwrap();
        let mut probe = model.clone();
        let result = probe.apply_game(&Edit::set(field.clone(), value));
        let refused = matches!(result, Err(EditError::ReadOnly { .. }));
        assert_eq!(model.field_locked(&field).unwrap(), refused, "{field}");
        if !refused {
            assert!(result.is_ok(), "{field}");
        }
    }
}

#[test]
fn row_locked_agrees_with_the_game_remove() {
    let mut model = standard_model();
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        for key in model.rows(&FieldId::new(list)).unwrap() {
            let target = row(list, key);
            let locked = model.row_locked(&target).unwrap();
            let refused = matches!(model.clone().apply_game(&Edit::remove(target)), Err(EditError::ReadOnly { .. }));
            assert_eq!(locked, refused, "{list}.{key}");
        }
    }
}

#[test]
fn placeholder_worn_and_character_rows_are_locked_free_rows_are_not() {
    let mut model = standard_model();
    for key in PLACEHOLDERS {
        assert!(model.row_locked(&row(names::INVENTORY, key)).unwrap(), "placeholder {key}");
    }
    // 60 sits in both hands of character 1, 91 in one hand of character 2.
    for key in [60, 91, 105] {
        assert!(model.row_locked(&row(names::INVENTORY, key)).unwrap(), "worn {key}");
    }
    let unworn = ids(TableKind::Weapon)[10];
    assert!(!model.row_locked(&row(names::INVENTORY, unworn)).unwrap());
    // A key item row removes, only its count is fixed.
    assert!(!model.row_locked(&row(names::KEY_ITEMS, key_items()[0])).unwrap());
    assert!(model.row_locked(&row(names::CHARACTER, 1)).unwrap());
    assert!(matches!(model.row_locked(&row(names::CHARACTER, 9)), Err(EditError::MissingRow { .. })));
    assert!(matches!(model.row_locked(&row(names::INVENTORY, 999_999)), Err(EditError::MissingRow { .. })));
    assert!(matches!(model.row_locked(&row("gil", 1)), Err(EditError::NotACollection { .. })));
}

#[test]
fn a_row_the_editor_cannot_rewrite_is_locked() {
    let mut data = SlotData::standard();
    let unworn = ids(TableKind::Weapon)[10];
    let index = data
        .inventory
        .iter()
        .position(|row| row.text.contains(&format!("\"contentId\":{unworn},")))
        .unwrap();
    data.inventory[index] = common::ff2_save::Row::raw(format!("{{ \"contentId\":{unworn}, \"count\":1 }}"));
    let mut model = common::ff2_save::model(&data.render());
    assert!(model.row_locked(&row(names::INVENTORY, unworn)).unwrap());
}

#[test]
fn a_key_item_keeps_a_count_of_one() {
    let mut model = standard_model();
    let free = ids(TableKind::KeyItem).into_iter().find(|id| !key_items().contains(id)).unwrap();
    let text = model.document().to_json();
    // Set: refused as locked; Insert with another count: OutOfRange at 1.
    let held = key_items()[0];
    let result = model.apply_game(&Edit::set(count(names::KEY_ITEMS, held), Value::Int(2)));
    assert!(matches!(result, Err(EditError::ReadOnly { .. })));
    for bad in [0, 2, 99] {
        let insert = Edit::insert(row(names::KEY_ITEMS, free), None, vec![(names::COUNT, Value::Int(bad))]);
        let result = model.apply_game(&insert);
        assert!(matches!(result, Err(EditError::OutOfRange { min: 1, max: 1, .. })), "count {bad}");
    }
    assert_eq!(model.document().to_json(), text);
    let insert = Edit::insert(row(names::KEY_ITEMS, free), None, vec![(names::COUNT, Value::Int(1))]);
    model.apply_game(&insert).unwrap();
    assert_eq!(model.get(&count(names::KEY_ITEMS, free)).unwrap(), Value::Int(1));
    // The raw interface holds no such rule.
    let raw = Edit::set(count(names::KEY_ITEMS, held), Value::Int(2));
    assert!(model.apply(&raw, CapPolicy::Enforce).is_ok());
}

#[test]
fn an_inventory_placeholder_row_refuses_every_edit() {
    let mut model = standard_model();
    for key in PLACEHOLDERS {
        let set = Edit::set(count(names::INVENTORY, key), Value::Int(7));
        assert!(matches!(model.apply_game(&set), Err(EditError::ReadOnly { .. })), "set {key}");
        let remove = Edit::remove(row(names::INVENTORY, key));
        assert!(matches!(model.apply_game(&remove), Err(EditError::ReadOnly { .. })), "remove {key}");
        let insert = Edit::insert(row(names::INVENTORY, key), None, Vec::new());
        assert!(matches!(model.apply_game(&insert), Err(EditError::ReadOnly { .. })), "insert {key}");
    }
}

#[test]
fn every_key_item_of_the_private_slots_holds_one() {
    let Some(slots) = ff2_real::slots() else { return };
    for slot in slots {
        let mut model = ff2_real::model(&slot);
        let list = FieldId::new(names::KEY_ITEMS);
        let Ok(keys) = model.rows(&list) else {
            panic!("{}: key item rows do not read", slot.label)
        };
        for key in keys {
            let found = model.get(&count(names::KEY_ITEMS, key));
            assert!(matches!(found, Ok(Value::Int(1))), "{}: {}", slot.label, count(names::KEY_ITEMS, key));
        }
    }
}
