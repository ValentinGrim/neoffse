//! Reads of the FF I model on the private slot files. The folder is read-only
//! input and the tests write nothing. A failure names the label and a field
//! path, never a value, a length or a name: the files hold personal data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::HashSet;

use common::ff1_ability_rules::ability_violations;
use common::ff1_real::{RealSlot, must, must_some, slots};
use common::ff1_rules::{cap_violations, violations};
use common::json_tree::Tree;
use neoffse_core::{FieldId, Shape, TableKind, Value};
use neoffse_format_pr::ff1::{Ff1Model, GAME, names};

fn tables() -> &'static neoffse_data::GameTables {
    must_some(neoffse_data::tables_for(GAME), "no FF I tables")
}

fn rows(model: &mut Ff1Model, list: &'static str, label: &str) -> Vec<u32> {
    must(model.rows(&FieldId::new(list)), &format!("{label}: {list} rows do not read"))
}

/// Every character field of every character, `gil` and every item count.
fn all_fields(model: &mut Ff1Model, label: &str) -> Vec<FieldId> {
    let mut fields = vec![FieldId::new(names::GIL)];
    let Ok(ids) = model.rows(&FieldId::new(names::CHARACTER)) else {
        panic!("{label}: character rows do not read")
    };
    for id in ids {
        fields.extend(names::character_fields(id));
    }
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        let collection = FieldId::new(list);
        let Ok(keys) = model.rows(&collection) else {
            panic!("{label}: {list} rows do not read")
        };
        fields.extend(keys.into_iter().map(|key| collection.clone().with_id(key).with_name(names::COUNT)));
    }
    fields
}

fn for_each_slot(mut check: impl FnMut(&RealSlot, &mut Ff1Model)) {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model();
        check(&slot, &mut model);
    }
}

fn int(model: &mut Ff1Model, label: &str, field: &FieldId) -> i64 {
    match model.get(field) {
        Ok(Value::Int(value)) => value,
        _ => panic!("{label}: {field} does not read as an integer"),
    }
}

#[test]
fn real_slots_read_every_declared_field() {
    for_each_slot(|slot, model| {
        for field in all_fields(model, &slot.label) {
            let Ok(value) = model.get(&field) else {
                panic!("{}: {field} does not read", slot.label)
            };
            let meta = must_some(model.field_meta(&field), &format!("{}: {field} has no meta", slot.label));
            if let (Value::Int(number), Shape::Int { min, max }) = (value, meta.shape) {
                assert!((min..=max).contains(&number), "{}: {field} is outside its storage bound", slot.label);
            }
        }
    });
}

#[test]
fn real_values_sit_inside_their_game_caps() {
    for_each_slot(|slot, model| {
        let found = cap_violations(model);
        assert!(
            found.is_empty(),
            "{}: {} (the cap reading of pixel-remaster.md#bounds-in-the-master-data may be wrong)",
            slot.label,
            found.join("; ")
        );
    });
}

#[test]
fn real_slots_obey_the_inventory_rules() {
    for_each_slot(|slot, model| {
        let found = violations(model, false);
        assert!(found.is_empty(), "{}: {}", slot.label, found.join("; "));
    });
}

#[test]
fn real_slot_counts_copy_the_inventory_count() {
    for_each_slot(|slot, model| {
        let found: Vec<String> = violations(model, true).into_iter().filter(|text| text.starts_with("copy:")).collect();
        assert!(
            found.is_empty(),
            "{}: {} (the [I] reading of pixel-remaster.md#equipment-slots needs a second look)",
            slot.label,
            found.join("; ")
        );
    });
}

#[test]
fn real_characters_have_unique_ids_and_known_jobs() {
    for_each_slot(|slot, model| {
        let ids = rows(model, names::CHARACTER, &slot.label);
        assert!(ids.iter().collect::<HashSet<_>>().len() == ids.len(), "{}: a character id repeats", slot.label);
        for id in ids {
            let field = FieldId::new(names::CHARACTER).with_id(id).with_name(names::JOB);
            let job = u32::try_from(int(model, &slot.label, &field)).unwrap_or(0);
            assert!(tables().entry(TableKind::Job, job).is_some(), "{}: {field} is no job of the table", slot.label);
        }
    });
}

#[test]
fn real_equipment_resolves_in_the_slot_tables() {
    let kinds = [TableKind::Weapon, TableKind::Shield, TableKind::Armor, TableKind::Helmet, TableKind::Gloves];
    for_each_slot(|slot, model| {
        for id in rows(model, names::CHARACTER, &slot.label) {
            for (number, kind) in (1..).zip(kinds) {
                let field = FieldId::new(names::CHARACTER).with_id(id).with_name(names::EQUIPMENT).with_id(number);
                let content = u32::try_from(int(model, &slot.label, &field)).unwrap_or(0);
                assert!(tables().entry(kind, content).is_some(), "{}: {field} is no entry of its slot table", slot.label);
            }
        }
    });
}

#[test]
fn real_inventory_ids_are_table_entries() {
    for_each_slot(|slot, model| {
        let inventory = rows(model, names::INVENTORY, &slot.label);
        let key_items = rows(model, names::KEY_ITEMS, &slot.label);
        for (list, keys) in [(names::INVENTORY, &inventory), (names::KEY_ITEMS, &key_items)] {
            assert!(keys.iter().collect::<HashSet<_>>().len() == keys.len(), "{}: a {list} key repeats", slot.label);
        }
        for key in inventory {
            let entry = tables().content(key);
            assert!(
                entry.is_some_and(|entry| entry.kind != TableKind::KeyItem),
                "{}: inventory.{key} is no item entry",
                slot.label
            );
        }
        for key in key_items {
            assert!(
                tables().entry(TableKind::KeyItem, key).is_some(),
                "{}: key_items.{key} is no key item entry",
                slot.label
            );
        }
    });
}

#[test]
fn real_reads_leave_the_text_unchanged() {
    for_each_slot(|slot, model| {
        for field in all_fields(model, &slot.label) {
            must(model.get(&field), &format!("{}: {field} does not read", slot.label));
        }
        assert!(model.document().to_json() == slot.json, "{}: a read changed the text", slot.label);
    });
}

#[test]
fn real_slots_obey_the_ability_rules() {
    for_each_slot(|slot, _| {
        let found = ability_violations(&Tree::parse(&slot.json));
        assert!(found.is_empty(), "{}: {}", slot.label, found.join("; "));
    });
}
