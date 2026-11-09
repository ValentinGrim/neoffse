//! Metadata of the FF II fields: labels, caps, slot tables, the computed
//! fields and the read-only person.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff2_save::standard_model;
use common::{STORAGE_MAX, character_field};
use neoffse_core::{FieldId, Shape, TableKind};
use neoffse_format_pr::ff2::names;

#[test]
fn every_character_field_has_its_label() {
    let model = standard_model();
    let labels = [
        "Name",
        "Person",
        "HP",
        "Max HP gain",
        "MP",
        "Max MP gain",
        "Power gain",
        "Vitality gain",
        "Agility gain",
        "Intelligence gain",
        "Spirit gain",
        "Magic gain",
        "Sword",
        "Knife",
        "Spear",
        "Axe",
        "Staff",
        "Bow",
        "Shield",
        "Bare hands",
        "Evasion",
        "Magic evasion",
        "Hand 1",
        "Count in slot 1",
        "Hand 2",
        "Count in slot 2",
        "Body",
        "Count in slot 3",
        "Head",
        "Count in slot 4",
        "Arms",
        "Count in slot 5",
    ];
    let fields = names::character_fields(1);
    assert_eq!(fields.len(), labels.len());
    for (field, label) in fields.iter().zip(labels) {
        assert_eq!(model.field_meta(field).unwrap().label, label, "{field}");
    }
}

#[test]
fn skill_caps_are_the_curve_top_and_the_evasion_top() {
    let model = standard_model();
    for key in 0..names::SKILLS {
        let field = character_field(1, names::SKILL).with_id(key);
        let meta = model.field_meta(&field).unwrap();
        let cap = if key < 8 { 860 } else { 1_500 };
        assert_eq!(meta.cap, Some(cap), "{field}");
        assert_eq!(meta.shape, Shape::Int { min: 0, max: STORAGE_MAX }, "{field}");
        assert!(!meta.read_only && meta.tables.is_empty(), "{field}");
    }
}

#[test]
fn hp_mp_and_gil_caps_come_from_the_tables() {
    let model = standard_model();
    let cap = |field: FieldId| model.field_meta(&field).unwrap().cap;
    assert_eq!(cap(character_field(1, names::HP)), Some(9_999));
    assert_eq!(cap(character_field(1, names::MP)), Some(999));
    assert_eq!(cap(FieldId::new(names::GIL)), Some(9_999_999));
    assert_eq!(cap(FieldId::new(names::INVENTORY).with_id(60).with_name(names::COUNT)), Some(99));
    assert_eq!(cap(FieldId::new(names::KEY_ITEMS).with_id(2).with_name(names::COUNT)), None);
}

#[test]
fn the_gains_carry_no_cap() {
    let model = standard_model();
    for name in [
        names::MAX_HP_GAIN,
        names::MAX_MP_GAIN,
        names::POWER,
        names::VITALITY,
        names::AGILITY,
        names::INTELLIGENCE,
        names::SPIRIT,
        names::MAGIC,
    ] {
        let meta = model.field_meta(&character_field(1, name)).unwrap();
        assert_eq!(meta.cap, None, "{name}");
        assert_eq!(meta.shape, Shape::Int { min: 0, max: STORAGE_MAX }, "{name}");
        assert!(!meta.read_only, "{name}");
    }
}

#[test]
fn slot_tables_follow_the_slots() {
    let model = standard_model();
    let hands = vec![TableKind::Weapon, TableKind::Shield];
    let tables = |slot| model.field_meta(&character_field(1, names::EQUIPMENT).with_id(slot)).unwrap().tables;
    assert_eq!(tables(1), hands);
    assert_eq!(tables(2), hands);
    assert_eq!(tables(3), [TableKind::Armor]);
    assert_eq!(tables(4), [TableKind::Helmet]);
    assert_eq!(tables(5), [TableKind::Gloves]);
    let count = model
        .field_meta(&character_field(1, names::EQUIPMENT).with_id(3).with_name(names::COUNT))
        .unwrap();
    assert!(count.tables.is_empty() && count.cap.is_none());
}

#[test]
fn the_computed_fields_are_read_only_and_capped() {
    let model = standard_model();
    for (name, label, cap) in [(names::MAX_HP, "Max HP", 9_999), (names::MAX_MP, "Max MP", 999)] {
        let meta = model.field_meta(&character_field(1, name)).unwrap();
        assert_eq!(meta.label, label, "{name}");
        assert_eq!(meta.shape, Shape::Int { min: 0, max: STORAGE_MAX }, "{name}");
        assert_eq!(meta.cap, Some(cap), "{name}");
        assert!(meta.read_only, "{name}");
        assert!(meta.tables.is_empty() && meta.names.is_empty(), "{name}");
    }
}

#[test]
fn the_person_is_read_only_with_the_job_table() {
    let model = standard_model();
    let meta = model.field_meta(&character_field(1, names::JOB)).unwrap();
    assert!(meta.read_only);
    assert_eq!(meta.tables, [TableKind::Job]);
    assert_eq!(meta.cap, None);
}

#[test]
fn the_view_lists_are_the_vocabulary_without_the_computed_fields() {
    let stored = names::character_fields(2);
    let derived = names::derived_fields(2);
    let party = names::party_fields(2);
    assert_eq!((stored.len(), derived.len(), party.len()), (32, 2, 34));
    assert!(derived.iter().all(|field| !stored.contains(field)));
    assert_eq!(derived, [character_field(2, names::MAX_HP), character_field(2, names::MAX_MP)]);
    let position = |name| party.iter().position(|field| *field == character_field(2, name)).unwrap();
    assert_eq!(position(names::MAX_HP), position(names::HP) + 1);
    assert_eq!(position(names::MAX_MP), position(names::MP) + 1);
    let without: Vec<FieldId> = party.into_iter().filter(|field| !derived.contains(field)).collect();
    assert_eq!(without, stored);
}

#[test]
fn collections_have_their_meta() {
    let model = standard_model();
    for (id, label) in [
        (names::CHARACTER, "Characters"),
        (names::INVENTORY, "Items"),
        (names::KEY_ITEMS, "Key items"),
        (names::KEYWORDS, "Keywords"),
    ] {
        assert_eq!(model.collection_meta(&FieldId::new(id)).unwrap().label, label, "{id}");
    }
    assert!(model.collection_meta(&FieldId::new(names::GIL)).is_none());
    assert!(model.collection_meta(&FieldId::new("keyword")).is_none());
}

#[test]
fn only_the_spell_level_is_edited_through_the_game_interface_in_every_mode() {
    let model = standard_model();
    let level = model.field_meta(&names::spell_field(1, 0)).unwrap();
    assert!(level.game_edit && !level.read_only);
    let mut others = names::party_fields(1);
    others.push(FieldId::new(names::GIL));
    others.push(FieldId::new(names::INVENTORY).with_id(60).with_name(names::COUNT));
    others.push(names::place(1, names::ABILITY, 0, names::ABILITY_ID));
    others.push(names::order_place(1, names::ADDITION_ORDER, 0));
    others.push(names::order_place(1, names::SORT_ORDER, 0));
    for field in others {
        assert!(!model.field_meta(&field).unwrap().game_edit, "{field}");
    }
}
