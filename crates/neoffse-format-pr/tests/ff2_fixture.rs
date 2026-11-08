//! The committed FF II slot fixture, its generator and the game rules it obeys.
//!
//! The JSON is synthetic. Regenerate the file with
//! `NEOFFSE_WRITE_FIXTURES=1 cargo test -p neoffse-format-pr --test ff2_fixture write_fixture`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::HashMap;

use common::ff1_save::keys;
use common::ff2_save::{SlotData, ids, standard_model, tables};
use common::pr_files::{assert_fixture_matches, text_at, unique};
use common::slot_model::int;
use common::{VEHICLES, character_path, slot};
use neoffse_core::{FieldId, TableKind, Value};
use neoffse_data::GameTables;
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Model, names};

const FILE: &str = "pr/ff2-slot.json";

#[test]
fn the_committed_slot_matches_the_generator() {
    assert_fixture_matches(FILE, &SlotData::standard().render());
}

#[test]
fn write_fixture() {
    common::pr_files::write_fixture(FILE, &SlotData::standard().render());
}

#[test]
fn the_fixture_is_game_style_and_covers_the_cases() {
    let text = SlotData::standard().render();
    let mut document = Document::parse(&text).unwrap();
    assert_eq!(document.to_json(), text);
    // `docs/formats/pixel-remaster.md#slot-keys`
    let order = [
        "id",
        "pictureData",
        "userData",
        "configData",
        "dataStorage",
        "mapData",
        "timeStamp",
        "playTime",
        "clearFlag",
        "isCompleteFlag",
    ];
    assert_eq!(keys(&text), order);
    let user = text_at(&mut document, &[Segment::Key("userData")]);
    assert!(unique(&keys(&user)));
    for index in 0..4 {
        assert!(unique(&keys(&text_at(&mut document, &character_path(index, &[])))));
    }
    assert_eq!(document.array_len(&VEHICLES).unwrap(), 7);

    let mut model = standard_model();
    let name = model.get(&FieldId::new(names::CHARACTER).with_id(4).with_name(names::NAME)).unwrap();
    assert_eq!(name, Value::Text("Élan".to_owned()));
    assert!(!"Élan".is_ascii());
    // The hands of the first character hold the same weapon, the third has an empty hand.
    assert_eq!((int(&mut model, &slot(1, 1)), int(&mut model, &slot(1, 2))), (60, 60));
    assert_eq!(int(&mut model, &slot(3, 2)), 47);
    assert_eq!(model.rows(&FieldId::new(names::CHARACTER)).unwrap(), [1, 2, 3, 4]);
    assert_eq!(model.rows(&FieldId::new(names::KEY_ITEMS)).unwrap().len(), 8);
}

#[test]
fn the_party_has_three_members_and_an_empty_seat() {
    let text = SlotData::standard().render();
    let mut document = Document::parse(&text).unwrap();
    let corps = |seat: usize| {
        vec![
            Segment::Key("userData"),
            Segment::Doc,
            Segment::Key("corpsList"),
            Segment::Doc,
            Segment::Key("target"),
            Segment::Index(seat),
            Segment::Doc,
            Segment::Key("characterId"),
        ]
    };
    let seats: Vec<Scalar> = (0..4).map(|seat| document.get(&corps(seat)).unwrap()).collect();
    assert_eq!(seats, [1, 2, 3, 0].map(Scalar::Int));
    let enabled = |index| character_path(index, &[Segment::Doc, Segment::Key("isEnableCorps")]);
    let flags: Vec<Scalar> = (0..4).map(|index| document.get(&enabled(index)).unwrap()).collect();
    assert_eq!(flags, [true, true, true, false].map(Scalar::Bool));
}

/// The rows the document names, as the tables hold them
/// (`docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`).
#[test]
fn the_named_ids_are_the_rows_of_the_document() {
    let tables = tables();
    let placeholder = |id, kind| {
        let entry = tables.entry(kind, id).unwrap();
        assert!(entry.placeholder, "{id}");
    };
    placeholder(47, TableKind::Weapon);
    placeholder(208, TableKind::Shield);
    placeholder(209, TableKind::Armor);
    placeholder(210, TableKind::Helmet);
    placeholder(211, TableKind::Gloves);
    for (id, kind) in [(60, TableKind::Weapon), (91, TableKind::Weapon), (105, TableKind::Shield)] {
        let entry = tables.entry(kind, id).unwrap();
        assert!(!entry.placeholder, "{id}");
    }
    for status in 1..=4 {
        assert!(tables.base_stats(status).is_some(), "status {status}");
    }
    assert_eq!(tables.spell_chains(), 40);
}

fn gains_stay_under_the_caps(tables: &GameTables, model: &mut Ff2Model) -> Vec<String> {
    let mut found = Vec::new();
    for (index, id) in model.rows(&FieldId::new(names::CHARACTER)).unwrap().into_iter().enumerate() {
        let Scalar::Int(status) = model.read(&character_path(index, &[Segment::Doc, Segment::Key("characterStatusId")])).unwrap() else {
            panic!("character {id}: no status id")
        };
        let status = u32::try_from(status).unwrap();
        let base = tables.base_stats(status).unwrap();
        let gains = [
            (names::MAX_HP_GAIN, base.hp, "MAX_HP"),
            (names::MAX_MP_GAIN, base.mp, "MAX_MP"),
            (names::POWER, base.power, "MAX_STRNGTH"),
            (names::VITALITY, base.vitality, "MAX_VITALITY"),
            (names::AGILITY, base.agility, "MAX_AGILITY"),
            (names::INTELLIGENCE, base.intelligence, "MAX_INTELIGENCE"),
            (names::SPIRIT, base.spirit, "MAX_SPRIT"),
            (names::MAGIC, base.magic, "MAX_MAGIC"),
        ];
        for (name, base, cap) in gains {
            let gain = int(model, &FieldId::new(names::CHARACTER).with_id(id).with_name(name));
            if gain < 0 || gain + i64::from(base) > tables.bound(cap).unwrap() {
                found.push(format!("cap: character {id} {name} passes {cap}"));
            }
        }
    }
    found
}

/// The inventory floor and the slot `count` copy, counting slots
/// (`docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`).
fn equipment_violations(model: &mut Ff2Model) -> Vec<String> {
    let mut found = Vec::new();
    let inventory = FieldId::new(names::INVENTORY);
    let mut counts = HashMap::new();
    for key in model.rows(&inventory).unwrap() {
        counts.insert(i64::from(key), int(model, &inventory.clone().with_id(key).with_name(names::COUNT)));
    }
    let mut wearers: HashMap<i64, i64> = HashMap::new();
    let mut worn = Vec::new();
    for id in model.rows(&FieldId::new(names::CHARACTER)).unwrap() {
        for number in 1..=names::EQUIPMENT_SLOTS {
            let field = slot(id, number);
            let content = int(model, &field);
            let count = int(model, &field.clone().with_name(names::COUNT));
            *wearers.entry(content).or_default() += 1;
            worn.push((field, content, count));
        }
    }
    for (field, content, count) in worn {
        match counts.get(&content) {
            None => found.push(format!("inventory: {field} has no row")),
            Some(owned) if *owned < wearers[&content] => found.push(format!("inventory: {field} is worn more often than owned")),
            Some(owned) if *owned != count => found.push(format!("copy: {field}.count differs from the inventory")),
            Some(_) => {}
        }
    }
    found
}

#[test]
fn the_fixture_obeys_the_game_rules() {
    let tables = tables();
    let mut model = standard_model();
    assert_eq!(gains_stay_under_the_caps(tables, &mut model), Vec::<String>::new());
    assert_eq!(equipment_violations(&mut model), Vec::<String>::new());
    let inventory = model.rows(&FieldId::new(names::INVENTORY)).unwrap();
    assert_eq!(inventory[..5], [47, 208, 209, 210, 211]);
    for (key, count) in [(47, 10), (208, 5), (209, 5), (210, 11), (211, 11)] {
        let field = FieldId::new(names::INVENTORY).with_id(key).with_name(names::COUNT);
        assert_eq!(int(&mut model, &field), count, "{key}");
    }
    let key_items = FieldId::new(names::KEY_ITEMS);
    for key in model.rows(&key_items).unwrap() {
        assert!(
            matches!(tables.entry(TableKind::KeyItem, key), Some(entry) if !entry.placeholder),
            "key item {key}"
        );
        assert_eq!(int(&mut model, &key_items.clone().with_id(key).with_name(names::COUNT)), 1);
    }
    for kind in [TableKind::Item, TableKind::Weapon, TableKind::Armor] {
        assert!(ids(kind).iter().all(|id| tables.content(*id).is_some()));
    }
}

/// Every spell entry holds the row of its chain; only the one mismatched
/// pair differs from the proficiency total of its level
/// (`docs/formats/pixel-remaster.md#row-and-value-in-ff-ii`).
#[test]
fn the_spells_obey_the_chain_law_but_one() {
    let data = SlotData::standard();
    let tables = tables();
    let mut mismatched = 0;
    let mut seen = 0;
    for character in &data.characters {
        assert!(!character.spells.is_empty());
        for spell in &character.spells {
            let step = tables.spell_step(spell.ability_id()).unwrap();
            assert_eq!((step.chain, step.level), (spell.chain, spell.row_level));
            let total = i64::from(tables.skill_total(spell.row_level).unwrap());
            seen += 1;
            if spell.list_value != total || spell.dict_value != total {
                mismatched += 1;
                assert_eq!((spell.row_level, spell.list_value, spell.dict_value), (1, 456, 400));
            }
        }
    }
    assert_eq!((seen, mismatched), (24, 1));
}
