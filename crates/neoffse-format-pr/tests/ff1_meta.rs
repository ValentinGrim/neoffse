//! The metadata of the FF I fields and collections. The expected values sit
//! in this file as data, not derived from the code under test.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::standard_model;
use neoffse_core::{Capability, FieldId, FieldMeta, Game, Shape, TableKind, Value};
use neoffse_format_pr::ff1::{CAPABILITIES, GAME, names};

const STORAGE: i64 = 2_147_483_647;

fn character(name: &'static str) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(1).with_name(name)
}

fn level(name: &'static str, level: u32) -> FieldId {
    character(name).with_id(level)
}

fn slot(slot: u32) -> FieldId {
    character(names::EQUIPMENT).with_id(slot)
}

/// One row of the field table: label, shape bounds, cap, tables.
struct Row {
    field: FieldId,
    label: String,
    min: i64,
    cap: Option<i64>,
    tables: Vec<TableKind>,
}

fn row(field: FieldId, label: &str, cap: Option<i64>, tables: &[TableKind]) -> Row {
    Row {
        field,
        label: label.to_owned(),
        min: 0,
        cap,
        tables: tables.to_vec(),
    }
}

fn table() -> Vec<Row> {
    let mut rows = vec![
        row(FieldId::new(names::GIL), "Gil", Some(999_999), &[]),
        row(character(names::JOB), "Job", None, &[TableKind::Job]),
        row(character(names::LEVEL), "Level", Some(99), &[]),
        row(character(names::EXP), "Experience", Some(9_999_999), &[]),
        row(character(names::HP), "HP", Some(999), &[]),
        row(character(names::MAX_HP_GAIN), "Max HP gain", None, &[]),
    ];
    let stats = [
        (names::POWER, "Power"),
        (names::VITALITY, "Vitality"),
        (names::AGILITY, "Agility"),
        (names::INTELLIGENCE, "Intelligence"),
        (names::SPIRIT, "Spirit"),
        (names::LUCK, "Luck"),
        (names::MAGIC, "Magic"),
    ];
    rows.extend(stats.map(|(name, label)| row(character(name), label, None, &[])));
    for number in 1..=8 {
        rows.push(row(level(names::CHARGES, number), &format!("Charges, level {number}"), Some(9), &[]));
        rows.push(row(
            level(names::CHARGES_MAX_GAIN, number),
            &format!("Max charges gain, level {number}"),
            None,
            &[],
        ));
    }
    let slots = [
        ("Right hand", TableKind::Weapon),
        ("Left hand", TableKind::Shield),
        ("Body", TableKind::Armor),
        ("Head", TableKind::Helmet),
        ("Arms", TableKind::Gloves),
    ];
    for (number, (label, table)) in (1..).zip(slots) {
        rows.push(row(slot(number), label, None, &[table]));
        rows.push(row(slot(number).with_name(names::COUNT), &format!("Count in slot {number}"), None, &[]));
    }
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        let mut count = row(FieldId::new(list).with_id(45).with_name(names::COUNT), "Count", Some(99), &[]);
        count.min = 1;
        rows.push(count);
    }
    rows
}

#[test]
fn field_meta_matches_the_field_table_row_by_row() {
    let model = standard_model();
    let name = model.field_meta(&character(names::NAME)).unwrap();
    assert_eq!((name.label.as_str(), name.shape, name.cap), ("Name", Shape::Text, None));
    assert_eq!(name.tables, []);
    for row in table() {
        let meta = model.field_meta(&row.field).unwrap_or_else(|| panic!("{} has no meta", row.field));
        assert_eq!(meta.label, row.label, "{}", row.field);
        assert_eq!(meta.shape, Shape::Int { min: row.min, max: STORAGE }, "{}", row.field);
        assert_eq!(meta.cap, row.cap, "{}", row.field);
        assert_eq!(meta.tables, row.tables, "{}", row.field);
        assert!(meta.names.is_empty(), "{}", row.field);
    }
}

#[test]
fn read_only_is_false_for_every_field() {
    let model = standard_model();
    let mut fields = names::character_fields(1);
    fields.push(FieldId::new(names::GIL));
    for (list, key) in [(names::INVENTORY, 62), (names::INVENTORY, 89), (names::KEY_ITEMS, 45)] {
        fields.push(FieldId::new(list).with_id(key).with_name(names::COUNT));
    }
    assert_eq!(fields.len(), 43);
    for field in fields {
        assert!(!model.field_meta(&field).unwrap().read_only, "{field}");
    }
}

#[test]
fn caps_come_from_the_game_tables() {
    let model = standard_model();
    let tables = neoffse_data::tables_for(GAME).unwrap();
    let count = FieldId::new(names::INVENTORY).with_id(89).with_name(names::COUNT);
    let capped = [
        (FieldId::new(names::GIL), 999_999, "MAX_GIL"),
        (character(names::LEVEL), 99, "MAX_LV"),
        (character(names::EXP), 9_999_999, "MAX_EXP"),
        (character(names::HP), 999, "MAX_HP"),
        (level(names::CHARGES, 1), 9, "MAX_MAGICAL_TIMES"),
        (count, 99, "MAX_ITEM"),
    ];
    for (field, cap, key) in capped {
        assert_eq!(model.field_meta(&field).unwrap().cap, Some(cap), "{field}");
        assert_eq!(tables.bound(key), Some(cap), "{key}");
    }
}

#[test]
fn gains_stats_and_slot_counts_have_no_cap() {
    let model = standard_model();
    let mut fields = vec![
        character(names::MAX_HP_GAIN),
        level(names::CHARGES_MAX_GAIN, 1),
        slot(1).with_name(names::COUNT),
    ];
    for name in [
        names::POWER,
        names::VITALITY,
        names::AGILITY,
        names::INTELLIGENCE,
        names::SPIRIT,
        names::LUCK,
        names::MAGIC,
    ] {
        fields.push(character(name));
    }
    for field in fields {
        assert_eq!(model.field_meta(&field).unwrap().cap, None, "{field}");
    }
}

#[test]
fn equipment_tables_follow_the_slot() {
    let model = standard_model();
    let kinds = [TableKind::Weapon, TableKind::Shield, TableKind::Armor, TableKind::Helmet, TableKind::Gloves];
    for (number, kind) in (1..).zip(kinds) {
        assert_eq!(model.field_meta(&slot(number)).unwrap().tables, [kind], "slot {number}");
    }
}

fn count_row(meta: &FieldMeta) {
    assert_eq!(meta.shape, Shape::Int { min: 1, max: STORAGE });
    assert_eq!(meta.cap, Some(99));
}

#[test]
fn collection_meta_of_the_three_collections() {
    let model = standard_model();
    let characters = model.collection_meta(&FieldId::new(names::CHARACTER)).unwrap();
    assert_eq!(characters.label, "Characters");
    assert!(!characters.insertable && !characters.removable);
    assert_eq!(characters.fields.len(), 0);

    let inventory = model.collection_meta(&FieldId::new(names::INVENTORY)).unwrap();
    assert_eq!(inventory.label, "Items");
    assert!(inventory.insertable && inventory.removable);
    assert_eq!(inventory.key.shape, Shape::Int { min: 1, max: STORAGE });
    assert_eq!(
        inventory.key.tables,
        [
            TableKind::Item,
            TableKind::Weapon,
            TableKind::Shield,
            TableKind::Armor,
            TableKind::Helmet,
            TableKind::Gloves
        ]
    );

    let key_items = model.collection_meta(&FieldId::new(names::KEY_ITEMS)).unwrap();
    assert_eq!(key_items.label, "Key items");
    assert!(key_items.insertable && key_items.removable);
    assert_eq!(key_items.key.shape, Shape::Int { min: 1, max: STORAGE });
    assert_eq!(key_items.key.tables, [TableKind::KeyItem]);

    for collection in [&inventory, &key_items] {
        assert_eq!(collection.fields.len(), 1);
        assert_eq!(collection.fields[0].name, names::COUNT);
        assert_eq!(collection.fields[0].initial, Value::Int(1));
        count_row(&collection.fields[0].meta);
    }
    assert!(model.collection_meta(&FieldId::new(names::GIL)).is_none());
    assert!(model.collection_meta(&FieldId::new(names::INVENTORY).with_id(7)).is_none());
}

#[test]
fn unknown_paths_have_no_meta() {
    let model = standard_model();
    for path in [
        character("nope"),
        level(names::CHARGES, 0),
        level(names::CHARGES, 9),
        level(names::CHARGES_MAX_GAIN, 9),
        slot(0),
        slot(6),
        slot(1).with_name("nope"),
        FieldId::new(names::INVENTORY).with_id(89).with_name(names::NAME),
        FieldId::new(names::INVENTORY),
        FieldId::new(names::GIL).with_id(1),
        FieldId::new("nothing"),
    ] {
        assert!(model.field_meta(&path).is_none(), "{path}");
    }
}

#[test]
fn a_path_that_names_an_absent_row_still_has_a_meta() {
    let model = standard_model();
    assert!(model.field_meta(&FieldId::new(names::INVENTORY).with_id(999).with_name(names::COUNT)).is_some());
    assert!(model.field_meta(&FieldId::new(names::CHARACTER).with_id(9).with_name(names::LEVEL)).is_some());
}

#[test]
fn character_fields_lists_39_ids_in_display_order() {
    let mut want: Vec<String> = [
        "name",
        "job",
        "level",
        "exp",
        "hp",
        "max_hp_gain",
        "power",
        "vitality",
        "agility",
        "intelligence",
        "spirit",
        "luck",
        "magic",
    ]
    .iter()
    .map(|name| format!("character.2.{name}"))
    .collect();
    want.extend((1..=8).map(|number| format!("character.2.charges.{number}")));
    want.extend((1..=8).map(|number| format!("character.2.charges_max_gain.{number}")));
    for number in 1..=5 {
        want.push(format!("character.2.equipment.{number}"));
        want.push(format!("character.2.equipment.{number}.count"));
    }
    let fields = names::character_fields(2);
    assert_eq!(fields.iter().map(ToString::to_string).collect::<Vec<_>>(), want);

    let mut model = standard_model();
    let mut labels = std::collections::HashSet::new();
    for field in fields {
        let meta = model.field_meta(&field).unwrap_or_else(|| panic!("{field} has no meta"));
        model.get(&field).unwrap_or_else(|error| panic!("{field}: {error}"));
        assert!(labels.insert(meta.label.clone()), "{field}: label repeats");
    }
}

#[test]
fn capabilities_and_game_constants() {
    assert_eq!(GAME, Game::Ff1Pr);
    assert_eq!(CAPABILITIES, [Capability::Party, Capability::Inventory]);
}
