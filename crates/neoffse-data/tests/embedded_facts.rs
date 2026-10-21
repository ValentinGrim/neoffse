//! Facts of the committed FF I tables (build 20249397): equipment, content
//! ids, jobs, bestiary, names and caps. Formats are in
//! `docs/formats/pixel-remaster.md`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1;
use neoffse_core::TableKind;
use neoffse_data::Language;

const CONTENT_KINDS: [TableKind; 7] = [
    TableKind::Item,
    TableKind::KeyItem,
    TableKind::Weapon,
    TableKind::Shield,
    TableKind::Armor,
    TableKind::Helmet,
    TableKind::Gloves,
];

fn names(kind: TableKind, id: u32) -> (&'static str, &'static str) {
    let entry = ff1().entry(kind, id).unwrap();
    (entry.name_en.as_str(), entry.name_fr.as_str())
}

#[test]
fn ice_shield_and_flame_shield() {
    // #equipment-slots
    assert_eq!(names(TableKind::Shield, 165), ("Ice Shield", "Bouclier gelé"));
    assert_eq!(names(TableKind::Shield, 166), ("Flame Shield", "Bouclier igné"));
}

#[test]
fn ids_the_go_editor_lacked() {
    let kind = |id| ff1().content(id).unwrap().kind;
    assert_eq!(names(TableKind::Weapon, 67).0, "Cat Claws");
    assert_eq!(names(TableKind::Weapon, 100).0, "Sasuke's Blade");
    assert_eq!(names(TableKind::Weapon, 103).0, "Masamune");
    assert_eq!(kind(67), TableKind::Weapon);
    assert_eq!(kind(100), TableKind::Weapon);
    assert_eq!(kind(103), TableKind::Weapon);
    assert_eq!(kind(151), TableKind::Armor);
    assert_eq!(kind(166), TableKind::Shield);
}

#[test]
fn a_wrong_kind_finds_nothing() {
    // Id 165 is a shield: ids are global to the content space, not to the API.
    assert!(ff1().entry(TableKind::Weapon, 165).is_none());
    assert!(ff1().entry(TableKind::Armor, 165).is_none());
    assert_eq!(ff1().content(165).unwrap().kind, TableKind::Shield);
}

#[test]
fn placeholder_per_equipment_kind() {
    // #id-spaces-and-tables, #equipment-slots
    let weapon = ff1().placeholder(TableKind::Weapon).unwrap();
    assert_eq!((weapon.id, weapon.name_en.as_str()), (62, "Empty"));
    for (kind, id) in [
        (TableKind::Shield, 130),
        (TableKind::Armor, 131),
        (TableKind::Helmet, 132),
        (TableKind::Gloves, 133),
    ] {
        let entry = ff1().placeholder(kind).unwrap();
        assert_eq!(entry.id, id);
        assert_eq!(entry.kind, kind);
        assert_eq!((entry.name_en.as_str(), entry.name_fr.as_str()), ("None", "---"));
    }
    assert!(ff1().placeholder(TableKind::Item).is_none());
    assert!(ff1().placeholder(TableKind::KeyItem).is_none());
    let flagged: usize = CONTENT_KINDS
        .iter()
        .map(|kind| ff1().table(*kind).unwrap().iter().filter(|entry| entry.placeholder).count())
        .sum();
    assert_eq!(flagged, 5);
}

#[test]
fn key_items_are_ids_45_to_61() {
    // #id-spaces-and-tables
    let ids: Vec<u32> = ff1().table(TableKind::KeyItem).unwrap().iter().map(|entry| entry.id).collect();
    assert_eq!(ids, (45..=61).collect::<Vec<u32>>());
}

#[test]
fn jobs_in_both_languages() {
    assert_eq!(names(TableKind::Job, 1), ("Warrior", "Guerrier"));
    assert_eq!(names(TableKind::Job, 7), ("Knight", "Paladin"));
    assert_eq!(names(TableKind::Job, 12), ("Black Wizard", "Sage noir"));
}

#[test]
fn the_bestiary_has_128_monsters() {
    // #bestiary, "FF I key space"
    let ids: Vec<u32> = ff1().table(TableKind::Monster).unwrap().iter().map(|entry| entry.id).collect();
    assert_eq!(ids, (1..=128).collect::<Vec<u32>>());
    assert_eq!(names(TableKind::Monster, 128).0, "Chaos");
    assert_eq!(names(TableKind::Monster, 61).0, "Vampire");
    assert!(ff1().entry(TableKind::Monster, 129).is_none());
}

#[test]
fn language_picks_the_name() {
    let ether = ff1().content(5).unwrap();
    assert_eq!(ether.name(Language::En), "Ether");
    assert_eq!(ether.name(Language::Fr), "Éther");
    assert_eq!(Language::ALL, [Language::En, Language::Fr]);
}

#[test]
fn bounds_match_the_master_data() {
    // #bounds-in-the-master-data
    for (key, value) in [
        ("MAX_GIL", 999_999),
        ("MAX_ITEM", 99),
        ("MAX_LV", 99),
        ("MAX_EXP", 9_999_999),
        ("MAX_HP", 999),
        ("MAX_MAGICAL_TIMES", 9),
        ("MAX_STRNGTH", 99),
        ("MAX_VITALITY", 99),
        ("MAX_AGILITY", 99),
        ("MAX_INTELIGENCE", 99),
        ("MAX_SPRIT", 99),
        ("MAX_LUCK", 99),
        ("MAX_ATTACK", 255),
        ("MAX_DEFENSE", 255),
        // The doc's row "and the matching rows" for ability attack, ability
        // defense, accuracy and evasion.
        ("MAX_ABILITY_ATTACK", 255),
        ("MAX_ABILITY_DEFENSE", 255),
        ("MAX_ACCURACY_RATE", 255),
        ("MAX_EVASION_RATE", 255),
    ] {
        assert_eq!(ff1().bound(key), Some(value), "{key}");
    }
}

#[test]
fn bound_keys_keep_the_game_spelling() {
    assert_eq!(ff1().bound("MAX_STRENGTH"), None);
    assert_eq!(ff1().bound("MAX_MP"), None);
    let keys: Vec<&str> = ff1().bounds().map(|(key, _)| key).collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn content_ids_run_from_2_to_202() {
    // #game-master-data: types 1 to 3 hold 121 content ids, from 2 to 202.
    let mut ids: Vec<u32> = CONTENT_KINDS
        .iter()
        .flat_map(|kind| ff1().table(*kind).unwrap().iter().map(|entry| entry.id))
        .collect();
    ids.sort_unstable();
    assert_eq!(ids.len(), 121);
    assert_eq!((ids.first(), ids.last()), (Some(&2), Some(&202)));
}

#[test]
fn the_crown_is_key_item_46() {
    // #game-master-data, "Item or key item"
    let crown = ff1().entry(TableKind::KeyItem, 46).unwrap();
    assert_eq!((crown.name_en.as_str(), crown.name_fr.as_str()), ("Crown", "Couronne"));
    assert!(!crown.placeholder);
}

#[test]
fn x_potion_nunchaku_and_knights_armor_are_real_rows() {
    // #go-editor-tables: content ids the Go editor lacked, none an empty slot.
    let found = |id| ff1().content(id).unwrap();
    assert_eq!((found(4).kind, found(4).name_en.as_str()), (TableKind::Item, "X-Potion"));
    assert_eq!((found(104).kind, found(104).name_en.as_str()), (TableKind::Weapon, "Nunchaku"));
    assert_eq!((found(105).kind, found(105).name_en.as_str()), (TableKind::Weapon, "Iron Nunchaku"));
    assert_eq!((found(151).kind, found(151).name_en.as_str()), (TableKind::Armor, "Knight's Armor"));
    for id in [4, 104, 105, 151, 166] {
        assert!(!found(id).placeholder, "content {id} is no empty slot");
    }
}

#[test]
fn commands_run_from_1_to_6_in_both_languages() {
    // #ability-job-and-command-tables
    let ids: Vec<u32> = ff1().table(TableKind::Command).unwrap().iter().map(|entry| entry.id).collect();
    assert_eq!(ids, (1..=6).collect::<Vec<u32>>());
    let french = ["Attaque", "Défense", "Magie", "Objets", "Fuite", "Retour"];
    for (index, name) in french.iter().enumerate() {
        let id = u32::try_from(index + 1).unwrap();
        assert_eq!(names(TableKind::Command, id).1, *name, "command {id}");
    }
}
