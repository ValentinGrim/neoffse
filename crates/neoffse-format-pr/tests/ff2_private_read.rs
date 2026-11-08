//! Open, read, summary and view of the FF II model on the private slot files.
//! The folder is read-only input and the tests write nothing. A failure names
//! the label and a field path, never a value, a length or a name: the files
//! hold personal data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::HashSet;

use common::ff1_real::{RealSlot, must, must_some};
use common::ff2_ability_rules::ability_violations;
use common::ff2_real::{model, slots};
use common::ff2_rules::{cap_violations, violations};
use common::json_tree::Tree;
use neoffse_core::{Capability, FieldId, SaveDocument, SaveFiles, Shape, TableKind, Value};
use neoffse_data::Language;
use neoffse_format_pr::ff2::{Ff2Format, Ff2Model, GAME, names};
use neoffse_format_pr::slots::slot_for_name;

fn tables() -> &'static neoffse_data::GameTables {
    must_some(neoffse_data::tables_for(GAME), "no FF II tables")
}

fn rows(model: &mut Ff2Model, list: &'static str, label: &str) -> Vec<u32> {
    must(model.rows(&FieldId::new(list)), &format!("{label}: {list} rows do not read"))
}

/// Every party field of every character, `gil` and every item count.
fn all_fields(model: &mut Ff2Model, label: &str) -> Vec<FieldId> {
    let mut fields = vec![FieldId::new(names::GIL)];
    for id in rows(model, names::CHARACTER, label) {
        fields.extend(names::party_fields(id));
    }
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        let collection = FieldId::new(list);
        fields.extend(
            rows(model, list, label)
                .into_iter()
                .map(|key| collection.clone().with_id(key).with_name(names::COUNT)),
        );
    }
    fields
}

fn for_each_slot(mut check: impl FnMut(&RealSlot, &mut Ff2Model)) {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = model(&slot);
        check(&slot, &mut model);
    }
}

fn int(model: &mut Ff2Model, label: &str, field: &FieldId) -> i64 {
    match model.get(field) {
        Ok(Value::Int(value)) => value,
        _ => panic!("{label}: {field} does not read as an integer"),
    }
}

#[test]
fn real_slots_open_as_ff2() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut files = SaveFiles::new();
        must(
            files.push(&slot.name, slot.bytes.clone()),
            &format!("{}: the file set refuses the name", slot.label),
        );
        let document = must(Ff2Format::new().open_document(&files), &format!("{}: the slot does not open", slot.label));
        assert!(document.game() == GAME, "{}: wrong game", slot.label);
        assert!(
            document.capabilities() == neoffse_format_pr::ff2::CAPABILITIES,
            "{}: wrong capabilities",
            slot.label
        );
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
            "{}: {} (the cap reading of pixel-remaster.md#character-growth-in-ff-ii may be wrong)",
            slot.label,
            found.join("; ")
        );
    });
}

#[test]
fn real_slots_obey_the_ability_rules() {
    for_each_slot(|slot, _| {
        let found = ability_violations(&Tree::parse(&slot.json));
        assert!(found.is_empty(), "{}: {}", slot.label, found.join("; "));
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
    let kinds: [&[TableKind]; 5] = [
        &[TableKind::Weapon, TableKind::Shield],
        &[TableKind::Weapon, TableKind::Shield],
        &[TableKind::Armor],
        &[TableKind::Helmet],
        &[TableKind::Gloves],
    ];
    for_each_slot(|slot, model| {
        for id in rows(model, names::CHARACTER, &slot.label) {
            for (number, accepted) in (1..).zip(kinds) {
                let field = FieldId::new(names::CHARACTER).with_id(id).with_name(names::EQUIPMENT).with_id(number);
                let content = u32::try_from(int(model, &slot.label, &field)).unwrap_or(0);
                assert!(
                    accepted.iter().any(|kind| tables().entry(*kind, content).is_some()),
                    "{}: {field} is no entry of its slot tables",
                    slot.label
                );
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
fn real_slots_have_a_summary() {
    for_each_slot(|slot, model| {
        let Ok(summary) = model.summary() else {
            panic!("{}: the summary does not read", slot.label)
        };
        let count = rows(model, names::CHARACTER, &slot.label).len();
        assert!(!summary.party.is_empty(), "{}: the party is empty", slot.label);
        assert!(summary.party.len() <= count, "{}: the party is longer than the roster", slot.label);
        assert!(summary.play_time.is_some(), "{}: no play time", slot.label);
        assert!(summary.slot.is_some(), "{}: no slot from the id", slot.label);
        assert!(summary.slot == slot_for_name(&slot.name), "{}: id and file name disagree", slot.label);
    });
}

#[test]
fn real_views_cover_every_row() {
    for_each_slot(|slot, model| {
        let Ok(view) = model.view(Language::En) else {
            panic!("{}: the view does not read", slot.label)
        };
        assert!(view.sections.len() == 3, "{}: section count", slot.label);
        let characters = rows(model, names::CHARACTER, &slot.label).len();
        let items = [names::INVENTORY, names::KEY_ITEMS, names::KEYWORDS]
            .into_iter()
            .map(|list| rows(model, list, &slot.label).len())
            .sum::<usize>();
        let party = must_some(view.section(Capability::Party), &format!("{}: no Party section", slot.label));
        let inventory = must_some(view.section(Capability::Inventory), &format!("{}: no Inventory section", slot.label));
        assert!(party.entities.len() == characters, "{}: party entities", slot.label);
        assert!(inventory.entities.len() == 1 + items, "{}: inventory entities", slot.label);
        let magic = must_some(view.section(Capability::Magic), &format!("{}: no Magic section", slot.label));
        // A Level entity has the id `character.<id>`; the raw pages follow it.
        let levels = magic.entities.iter().filter(|entity| entity.id.keys().len() == 2).count();
        assert!(levels == characters, "{}: magic entities", slot.label);
        for entity in &party.entities {
            assert!(entity.fields.len() == 34, "{}: {} holds a field count other than 34", slot.label, entity.id);
        }
        for entity in party.entities.iter().chain(&inventory.entities) {
            for field in &entity.fields {
                let Ok(value) = model.get(&field.id) else {
                    panic!("{}: {} does not read", slot.label, field.id)
                };
                assert!(value == field.value, "{}: {} differs from the model", slot.label, field.id);
            }
        }
        assert!(model.document().to_json() == slot.json, "{}: the view changed the text", slot.label);
    });
}
