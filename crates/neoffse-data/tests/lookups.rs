//! Lookups on the minimal sources: each rule stands apart from the committed data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{ABILITIES, CONTENT, minimal};
use neoffse_core::{Game, TableKind};
use neoffse_data::{AbilityKind, GameTables, Language, Sources};

fn tables() -> GameTables {
    GameTables::parse(Game::Ff1Pr, &minimal()).unwrap()
}

#[test]
fn entry_finds_by_kind_and_id() {
    let tables = tables();
    let entry = tables.entry(TableKind::Item, 5).unwrap();
    assert_eq!(entry.id, 5);
    assert_eq!(entry.kind, TableKind::Item);
    assert_eq!(tables.entry(TableKind::Weapon, 8).unwrap().name_en, "Dagger");
    assert_eq!(tables.entry(TableKind::Job, 2).unwrap().name_en, "Thief");
    assert_eq!(tables.entry(TableKind::Command, 2).unwrap().name_en, "Defend");
    assert_eq!(tables.entry(TableKind::Monster, 1).unwrap().name_en, "Goblin");
    let cure = tables.entry(TableKind::Ability, 3).unwrap();
    let info = cure.ability.unwrap();
    assert_eq!((info.content_id, info.kind, info.level), (400, AbilityKind::WhiteMagic, 1));
    assert!(tables.entry(TableKind::Item, 5).unwrap().ability.is_none());
}

#[test]
fn content_searches_the_seven_kinds() {
    let tables = tables();
    assert_eq!(tables.content(5).unwrap().kind, TableKind::Item);
    assert_eq!(tables.content(8).unwrap().kind, TableKind::Weapon);
    // 209 and 400 are ability content ids, not content-table ids.
    assert!(tables.content(209).is_none());
    assert!(tables.content(400).is_none());
    assert_eq!(tables.ability_by_content(400).unwrap().id, 3);
    assert!(tables.ability_by_content(5).is_none());
}

#[test]
fn a_missing_id_is_none() {
    let tables = tables();
    assert!(tables.entry(TableKind::Item, 6).is_none());
    assert!(tables.entry(TableKind::Item, 0).is_none());
    assert!(tables.content(6).is_none());
    // Present as a weapon, absent as an item.
    assert!(tables.entry(TableKind::Item, 8).is_none());
    assert!(tables.entry(TableKind::Armor, 8).is_none());
    assert!(tables.entry(TableKind::Monster, 3).is_none());
}

#[test]
fn table_is_ascending_and_keeps_placeholders() {
    let tables = tables();
    let weapons = tables.table(TableKind::Weapon).unwrap();
    assert_eq!(weapons.iter().map(|entry| entry.id).collect::<Vec<_>>(), [8, 9]);
    assert!(!weapons[0].placeholder);
    assert!(weapons[1].placeholder);
    assert_eq!(weapons[1].kind, TableKind::Weapon);
    assert_eq!(tables.placeholder(TableKind::Weapon).unwrap().id, 9);
    assert!(tables.placeholder(TableKind::Item).is_none());
    assert_eq!(tables.game(), Game::Ff1Pr);
}

#[test]
fn names_per_language_and_accents() {
    let tables = tables();
    let ether = tables.entry(TableKind::Item, 5).unwrap();
    assert_eq!(ether.name(Language::En), "Ether");
    assert_eq!(ether.name(Language::Fr), "Éther");
    assert_ne!(ether.name(Language::En), ether.name(Language::Fr));
    assert_eq!(tables.entry(TableKind::Command, 2).unwrap().name(Language::Fr), "Défense");
}

#[test]
fn bounds_are_read_by_key() {
    let tables = tables();
    assert_eq!(tables.bound("MAX_GIL"), Some(999_999));
    assert_eq!(tables.bound("MAX_MP"), None);
    let pairs: Vec<_> = tables.bounds().collect();
    assert_eq!(pairs, [("MAX_GIL", 999_999), ("MAX_HP", 999), ("MAX_LV", 4)]);
}

#[test]
fn crlf_text_parses_the_same() {
    let crlf: Vec<String> = [
        CONTENT,
        ABILITIES,
        common::JOBS,
        common::COMMANDS,
        common::MONSTERS,
        common::BOUNDS,
        common::GROWTH,
    ]
    .iter()
    .map(|text| text.replace('\n', "\r\n"))
    .collect();
    let sources = Sources {
        content: &crlf[0],
        abilities: &crlf[1],
        jobs: &crlf[2],
        commands: &crlf[3],
        monsters: Some(&crlf[4]),
        bounds: &crlf[5],
        growth: Some(&crlf[6]),
    };
    assert_eq!(GameTables::parse(Game::Ff1Pr, &sources).unwrap(), tables());
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let stripped: String = CONTENT
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .map(|line| line.to_owned() + "\n")
        .collect();
    let padded = format!("\n\n# one more comment\n\n{CONTENT}\n\n");
    for text in [stripped.as_str(), padded.as_str()] {
        let sources = Sources { content: text, ..minimal() };
        assert_eq!(GameTables::parse(Game::Ff1Pr, &sources).unwrap(), tables());
    }
}

#[test]
fn a_kind_without_a_file_has_no_table() {
    let tables = tables();
    assert!(tables.table(TableKind::Map).is_none());
    assert!(tables.entry(TableKind::Map, 1).is_none());
    assert!(tables.placeholder(TableKind::Map).is_none());
    assert!(tables.table(TableKind::Armor).is_none());
}
