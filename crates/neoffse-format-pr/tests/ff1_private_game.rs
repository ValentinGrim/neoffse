//! `apply_game` on the private slot files. The folder is read-only input:
//! edits happen in memory and the tests write nothing. A failure names the
//! label and a path, never a value.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_real::{RealSlot, slots};
use common::ff1_rules::assert_game_rules;
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, RowId, Value};
use neoffse_format_pr::container::{decode, encode};
use neoffse_format_pr::ff1::{Ff1Model, GAME, names};

const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;

fn slot_field(id: u32, number: u32) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id).with_name(names::EQUIPMENT).with_id(number)
}

fn int(model: &mut Ff1Model, field: &FieldId, label: &str) -> i64 {
    match model.get(field) {
        Ok(Value::Int(value)) => value,
        _ => panic!("{label}: {field} does not read as an integer"),
    }
}

fn first_two(model: &mut Ff1Model, label: &str) -> (u32, u32) {
    match model.rows(&FieldId::new(names::CHARACTER)).as_deref() {
        Ok([first, second, ..]) => (*first, *second),
        _ => panic!("{label}: fewer than two characters"),
    }
}

/// Every field the model maps that a game edit can move, with its value.
fn snapshot(model: &mut Ff1Model, label: &str) -> Vec<(FieldId, i64)> {
    let mut fields = Vec::new();
    for id in model.rows(&FieldId::new(names::CHARACTER)).unwrap() {
        fields.extend(
            names::character_fields(id)
                .into_iter()
                .filter(|field| !matches!(field.keys().get(2), Some(neoffse_core::Key::Name(names::NAME)))),
        );
    }
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        let collection = FieldId::new(list);
        for key in model.rows(&collection).unwrap() {
            fields.push(collection.clone().with_id(key).with_name(names::COUNT));
        }
    }
    fields.into_iter().map(|field| (field.clone(), int(model, &field, label))).collect()
}

fn run(model: &mut Ff1Model, set: &neoffse_core::ChangeSet) {
    let applied = set.apply_with(STORAGE_ONLY, |edit, policy| model.apply(edit, policy));
    assert!(applied.is_ok(), "the raw interface refuses a game change");
}

#[test]
fn real_equip_keeps_the_game_rules() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let label = &slot.label;
        let mut model = slot.model();
        let (first, second) = first_two(&mut model, label);
        assert_game_rules(&mut model, false);
        let wanted = int(&mut model, &slot_field(second, 1), label);
        let edit = Edit::set(slot_field(first, 1), Value::Int(wanted));
        let Ok(change) = model.apply_game(&edit) else {
            panic!("{label}: the equip is refused")
        };
        assert_game_rules(&mut model, false);
        let decoded = decode(&slot.bytes).unwrap_or_else(|_| panic!("{label}: decode failed"));
        let bytes = encode(&model.document().to_json(), &decoded.container).unwrap_or_else(|_| panic!("{label}: encode failed"));
        let again = decode(&bytes).unwrap_or_else(|_| panic!("{label}: second decode failed"));
        let mut rebuilt = RealSlot {
            label: label.clone(),
            bytes,
            json: again.json,
        }
        .model();
        assert_eq!(
            snapshot(&mut rebuilt, label),
            snapshot(&mut model, label),
            "{label}: reads after the round trip"
        );
        run(&mut rebuilt, &change.inverse);
        assert_eq!(rebuilt.document().to_json(), slot.json, "{label}: undo on the rebuilt model");
    }
}

#[test]
fn real_unequip_and_re_equip_restore_the_text() {
    let Some(slots) = slots() else { return };
    let Some(tables) = neoffse_data::tables_for(GAME) else {
        panic!("no FF I tables")
    };
    for slot in slots {
        let label = &slot.label;
        let mut model = slot.model();
        let (first, _) = first_two(&mut model, label);
        let old = int(&mut model, &slot_field(first, 1), label);
        let Some(placeholder) = tables.placeholder(neoffse_core::TableKind::Weapon) else {
            panic!("{label}: no weapon placeholder")
        };
        if u32::try_from(old).is_ok_and(|id| id == placeholder.id) {
            neoffse_testkit::skip(&format!("{label} holds the placeholder in slot 1"));
            continue;
        }
        let before = snapshot(&mut model, label);
        let out = Edit::set(slot_field(first, 1), Value::Int(i64::from(placeholder.id)));
        let back = Edit::set(slot_field(first, 1), Value::Int(old));
        let Ok(unequip) = model.apply_game(&out) else {
            panic!("{label}: the unequip is refused")
        };
        let Ok(equip) = model.apply_game(&back) else {
            panic!("{label}: the re-equip is refused")
        };
        assert_eq!(snapshot(&mut model, label), before, "{label}: reads after the round trip");
        run(&mut model, &equip.inverse);
        run(&mut model, &unequip.inverse);
        assert_eq!(model.document().to_json(), slot.json, "{label}: both undos");
    }
}

#[test]
fn real_worn_and_placeholder_rows_refuse_removal() {
    let Some(slots) = slots() else { return };
    let Some(tables) = neoffse_data::tables_for(GAME) else {
        panic!("no FF I tables")
    };
    for slot in slots {
        let label = &slot.label;
        let mut model = slot.model();
        let inventory = FieldId::new(names::INVENTORY);
        let rows = model.rows(&inventory).unwrap_or_else(|_| panic!("{label}: inventory rows do not read"));
        let mut worn = Vec::new();
        for character in model.rows(&FieldId::new(names::CHARACTER)).unwrap() {
            for number in 1..=names::EQUIPMENT_SLOTS {
                worn.push(int(&mut model, &slot_field(character, number), label));
            }
        }
        for key in rows {
            let locked = tables.content(key).is_some_and(|entry| entry.placeholder) || worn.contains(&i64::from(key));
            if !locked {
                continue;
            }
            let refused = model.apply_game(&Edit::remove(RowId::new(inventory.clone(), key)));
            assert!(matches!(refused, Err(EditError::ReadOnly { .. })), "{label}: inventory.{key} is removable");
            assert_eq!(model.document().to_json(), slot.json, "{label}: inventory.{key} changed the text");
        }
    }
}
