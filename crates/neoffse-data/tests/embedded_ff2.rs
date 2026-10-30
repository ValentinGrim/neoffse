//! The committed FF II tables (build 20251646), after
//! `docs/formats/pixel-remaster.md#master-data-per-game` and
//! `#rows-without-a-name`. Counts come from the document, not from the file.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;

use common::{game_tables, kind_rule::kind_of};
use neoffse_core::{Game, TableKind};
use neoffse_data::{AbilityKind, Entry, GameTables, Language, sources_for};

/// The content tables, in the order of the figures of the document.
const CONTENT: [TableKind; 8] = [
    TableKind::Item,
    TableKind::KeyItem,
    TableKind::Weapon,
    TableKind::Shield,
    TableKind::Armor,
    TableKind::Helmet,
    TableKind::Gloves,
    TableKind::Keyword,
];

fn ff2() -> &'static GameTables {
    game_tables(Game::Ff2Pr)
}

fn table(kind: TableKind) -> &'static [Entry] {
    ff2().table(kind).unwrap()
}

fn ids(kind: TableKind) -> Vec<u32> {
    table(kind).iter().map(|entry| entry.id).collect()
}

fn nameless(kind: TableKind) -> Vec<u32> {
    table(kind).iter().filter(|entry| entry.name_en.is_none()).map(|entry| entry.id).collect()
}

#[test]
fn game_is_ff2() {
    assert_eq!(ff2().game(), Game::Ff2Pr);
}

#[test]
fn a_fresh_parse_equals_the_cached_tables() {
    let fresh = GameTables::parse(Game::Ff2Pr, &sources_for(Game::Ff2Pr).unwrap()).unwrap();
    assert_eq!(&fresh, ff2());
}

#[test]
fn counts_per_table() {
    let content = [
        (TableKind::Item, 71),
        (TableKind::KeyItem, 14),
        (TableKind::Weapon, 54),
        (TableKind::Shield, 10),
        (TableKind::Armor, 21),
        (TableKind::Helmet, 11),
        (TableKind::Gloves, 11),
        (TableKind::Keyword, 15),
    ];
    for (kind, count) in content {
        assert_eq!(table(kind).len(), count, "{kind:?}");
    }
    assert_eq!(content.iter().map(|(_, count)| count).sum::<usize>(), 207);
    assert_eq!(table(TableKind::Ability).len(), 808);
    assert_eq!(table(TableKind::Job).len(), 9);
    assert_eq!(table(TableKind::Command).len(), 7);
}

#[test]
fn only_the_tables_ff2_has_exist() {
    for kind in [TableKind::Monster, TableKind::MagicStone, TableKind::CommandContent, TableKind::Map] {
        assert!(ff2().table(kind).is_none(), "{kind:?}");
        assert!(ff2().entry(kind, 1).is_none(), "{kind:?}");
    }
}

#[test]
fn ids_rise_inside_every_table() {
    for kind in CONTENT.into_iter().chain([TableKind::Ability, TableKind::Job, TableKind::Command]) {
        let list = ids(kind);
        assert!(list.windows(2).all(|pair| pair[0] < pair[1]), "{kind:?}");
    }
}

#[test]
fn documented_id_ranges() {
    assert_eq!(ids(TableKind::Job), (1..=9).collect::<Vec<_>>());
    assert_eq!(ids(TableKind::Command), (1..=7).collect::<Vec<_>>());
    // Contents 917 to 931, type_value 1 to 15 (`#master-data-per-game`, "Content types").
    assert_eq!(ids(TableKind::Keyword), (917..=931).collect::<Vec<_>>());
    // Content 1 is gil, which the file does not list. The first and last ids and the last ability id are
    // read in content.csv and ability.csv of build 20251646, not in the committed files.
    let all: Vec<u32> = CONTENT.iter().flat_map(|kind| ids(*kind)).collect();
    assert_eq!((all.iter().min(), all.iter().max()), (Some(&2), Some(&931)));
    let abilities = ids(TableKind::Ability);
    assert_eq!((abilities.first(), abilities.last()), (Some(&1), Some(&809)));
}

#[test]
fn the_empty_slot_rows() {
    // `#master-data-per-game`, "Equipment groups": one row per group 1 to 5.
    let slots = [
        (TableKind::Weapon, 47, "Empty"),
        (TableKind::Shield, 208, "None"),
        (TableKind::Armor, 209, "None"),
        (TableKind::Helmet, 210, "None"),
        (TableKind::Gloves, 211, "None"),
    ];
    for (kind, id, name) in slots {
        let entry = ff2().placeholder(kind).unwrap();
        assert_eq!((entry.id, entry.kind, entry.placeholder), (id, kind, true), "{kind:?}");
        assert_eq!(entry.name(Language::En), Some(name), "{kind:?}");
        assert_eq!(ff2().entry(kind, id), Some(entry));
    }
    let marked = ids_with_placeholder();
    assert_eq!(marked, [47, 208, 209, 210, 211]);
    for kind in [TableKind::Item, TableKind::KeyItem, TableKind::Keyword] {
        assert!(ff2().placeholder(kind).is_none(), "{kind:?}");
    }
}

fn ids_with_placeholder() -> Vec<u32> {
    let kinds = [TableKind::Weapon, TableKind::Shield, TableKind::Armor, TableKind::Helmet, TableKind::Gloves];
    let mut marked: Vec<u32> = kinds
        .iter()
        .flat_map(|kind| table(*kind).iter().filter(|entry| entry.placeholder).map(|entry| entry.id))
        .collect();
    marked.sort_unstable();
    marked
}

#[test]
fn keywords_are_named() {
    for entry in table(TableKind::Keyword) {
        assert!(entry.name_en.is_some() && entry.name_fr.is_some(), "keyword {}", entry.id);
        assert!(!entry.placeholder);
    }
    // A keyword is no inventory item.
    assert!(ff2().content(917).is_none());
    assert!(ff2().entry(TableKind::Keyword, 917).is_some());
}

#[test]
fn the_ability_type_ids_match_the_document() {
    // `#master-data-per-game`, "Abilities": 10 values.
    let mut histogram = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        *histogram.entry(entry.ability.unwrap().type_id).or_insert(0_usize) += 1;
    }
    let expected = BTreeMap::from([(1, 361), (2, 395), (3, 2), (4, 5), (5, 1), (6, 3), (7, 1), (8, 1), (12, 1), (13, 38)]);
    assert_eq!(histogram, expected);
    assert_eq!(histogram.values().sum::<usize>(), 808);
}

#[test]
fn the_ability_kinds_match_the_document() {
    let mut kinds = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        let info = entry.ability.unwrap();
        assert_eq!(kind_of(info.type_id), Some(info.kind), "ability {}", entry.id);
        *kinds.entry(format!("{:?}", info.kind)).or_insert(0_usize) += 1;
    }
    let expected: BTreeMap<String, usize> = [("WhiteMagic", 361), ("BlackMagic", 395), ("Monster", 40), ("Command", 11), ("Internal", 1)]
        .into_iter()
        .map(|(kind, count)| (kind.to_owned(), count))
        .collect();
    assert_eq!(kinds, expected);
}

#[test]
fn every_ability_has_level_0() {
    // FF I reads 1 to 8 for a spell; FF II stores 0 on every row.
    for entry in table(TableKind::Ability) {
        assert_eq!(entry.ability.unwrap().level, 0, "ability {}", entry.id);
    }
}

#[test]
fn five_abilities_have_no_name_and_the_rest_have_two() {
    let none = nameless(TableKind::Ability);
    assert_eq!(none.len(), 5);
    assert!(none.contains(&711));
    for entry in table(TableKind::Ability) {
        assert_eq!(entry.name_en.is_none(), entry.name_fr.is_none(), "ability {}", entry.id);
    }
    // Nothing else lacks a name: jobs, commands and content are all named.
    for kind in [TableKind::Job, TableKind::Command, TableKind::Item, TableKind::Weapon, TableKind::Keyword] {
        assert!(nameless(kind).is_empty(), "{kind:?}");
    }
}

#[test]
fn ability_711_is_two_sword_style() {
    // Kinds note, save anchor: every FF II character holds it, 226 above its content id.
    let entry = ff2().entry(TableKind::Ability, 711).unwrap();
    let info = entry.ability.unwrap();
    assert_eq!((info.type_id, info.kind, info.content_id), (12, AbilityKind::Internal, Some(937)));
    assert_eq!((entry.name_en.as_deref(), entry.name_fr.as_deref()), (None, None));
    assert_eq!(ff2().ability_by_content(937), Some(entry));
}

#[test]
fn content_ids_are_present_and_unique() {
    let mut seen = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        let content_id = entry.ability.unwrap().content_id.expect("FF II has no contentless ability");
        assert_ne!(content_id, 0);
        assert_eq!(seen.insert(content_id, entry.id), None, "content {content_id}");
    }
}

#[test]
fn the_bounds_match_the_document() {
    let tables = ff2();
    assert_eq!(tables.bounds().count(), 39);
    let named = [
        ("MAX_GIL", 9_999_999),
        ("MAX_ITEM", 99),
        ("MAX_LV", 99),
        ("MAX_EXP", 9_999_999),
        ("MAX_HP", 9_999),
        ("MAX_MP", 999),
        ("MAX_MAGICAL_LEVEL", 16),
        ("MAX_WEAPON_LEVEL", 16),
        ("MAX_ATTACK_SPEED", 36),
    ];
    for (key, value) in named {
        assert_eq!(tables.bound(key), Some(value), "{key}");
    }
    assert_eq!(tables.bound("MAX_MAGICAL_TIMES"), None);
    assert_eq!(tables.bound("MAX_GRADUAL_PETRIFICATION"), None);
    assert!(tables.bounds().all(|(key, value)| key.starts_with("MAX_") && value >= 0));
}

#[test]
fn there_are_no_growth_tables() {
    // character_status.csv holds 10 rows, but growth waits for per-game research.
    assert_eq!(ff2().base_hp(1), None);
    assert_eq!(ff2().base_vitality(1), None);
    assert_eq!(ff2().exp_to_reach(1), None);
    assert_eq!(ff2().growth_step(1, 2), None);
}

#[test]
fn a_content_id_is_no_ability_id() {
    // An id of another table finds nothing: 937 is a content id, not an ability id.
    assert!(ff2().entry(TableKind::Ability, 937).is_none());
    assert!(ff2().content(937).is_none());
}
