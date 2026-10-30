//! The committed FF IV tables (build 20265023), after
//! `docs/formats/pixel-remaster.md#master-data-per-game` and
//! `#rows-without-a-name`. Counts come from the document, not from the file.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;

use common::{game_tables, kind_rule::kind_of};
use neoffse_core::{Game, TableKind};
use neoffse_data::{Entry, GameTables, Language, sources_for};

/// The content tables, in the order of the figures of the document.
const CONTENT: [TableKind; 7] = [
    TableKind::Item,
    TableKind::KeyItem,
    TableKind::Weapon,
    TableKind::Shield,
    TableKind::Armor,
    TableKind::Helmet,
    TableKind::Gloves,
];

const EQUIPMENT: [TableKind; 5] = [TableKind::Weapon, TableKind::Shield, TableKind::Armor, TableKind::Helmet, TableKind::Gloves];

fn ff4() -> &'static GameTables {
    game_tables(Game::Ff4Pr)
}

fn table(kind: TableKind) -> &'static [Entry] {
    ff4().table(kind).unwrap()
}

fn ids(kind: TableKind) -> Vec<u32> {
    table(kind).iter().map(|entry| entry.id).collect()
}

fn nameless(kind: TableKind) -> Vec<u32> {
    table(kind).iter().filter(|entry| entry.name_en.is_none()).map(|entry| entry.id).collect()
}

#[test]
fn game_is_ff4() {
    assert_eq!(ff4().game(), Game::Ff4Pr);
}

#[test]
fn a_fresh_parse_equals_the_cached_tables() {
    let fresh = GameTables::parse(Game::Ff4Pr, &sources_for(Game::Ff4Pr).unwrap()).unwrap();
    assert_eq!(&fresh, ff4());
}

#[test]
fn counts_per_table() {
    // Weapon is 74 rows of parts_group_id 1 and 19 of group 6, read as one kind.
    let content = [
        (TableKind::Item, 61),
        (TableKind::KeyItem, 14),
        (TableKind::Weapon, 93),
        (TableKind::Shield, 13),
        (TableKind::Armor, 28),
        (TableKind::Helmet, 21),
        (TableKind::Gloves, 21),
    ];
    for (kind, count) in content {
        assert_eq!(table(kind).len(), count, "{kind:?}");
    }
    assert_eq!(content.iter().map(|(_, count)| count).sum::<usize>(), 251);
    assert_eq!(table(TableKind::Ability).len(), 446);
    assert_eq!(table(TableKind::Job).len(), 15);
    assert_eq!(table(TableKind::Command).len(), 35);
}

#[test]
fn only_the_tables_ff4_has_exist() {
    for kind in [
        TableKind::Monster,
        TableKind::Keyword,
        TableKind::MagicStone,
        TableKind::CommandContent,
        TableKind::Map,
    ] {
        assert!(ff4().table(kind).is_none(), "{kind:?}");
        assert!(ff4().entry(kind, 1).is_none(), "{kind:?}");
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
    assert_eq!(ids(TableKind::Job), (1..=15).collect::<Vec<_>>());
    // Inventory note, "Jobs and commands": command IDs 1-20, 22-28, 31, 34, 57-62.
    let commands: Vec<u32> = (1..=20).chain(22..=28).chain([31, 34]).chain(57..=62).collect();
    assert_eq!(ids(TableKind::Command), commands);
    // Measured in content.csv, weapon.csv and ability.csv of build 20265023, not in the committed
    // files. Content 1 is gil, which the file does not list. The 19 group 6 weapons span 196 to 521,
    // so content 521 exists only when group 6 reads as a weapon.
    let first_last = |kind| {
        let list = ids(kind);
        (list.first().copied(), list.last().copied())
    };
    assert_eq!(first_last(TableKind::Item), (Some(2), Some(520)));
    assert_eq!(first_last(TableKind::KeyItem), (Some(62), Some(75)));
    assert_eq!(first_last(TableKind::Weapon), (Some(127), Some(521)));
    let worn: Vec<u32> = EQUIPMENT[1..].iter().flat_map(|kind| ids(*kind)).collect();
    assert_eq!((worn.iter().min(), worn.iter().max()), (Some(&224), Some(&309)));
    assert_eq!(first_last(TableKind::Ability), (Some(1), Some(489)));
    let all: Vec<u32> = CONTENT.iter().flat_map(|kind| ids(*kind)).collect();
    assert_eq!((all.iter().min(), all.iter().max()), (Some(&2), Some(&521)));
}

#[test]
fn the_empty_slot_rows() {
    // `#master-data-per-game`, "Equipment groups": one row per group 1 to 5. Group 6 has none.
    let slots = [
        (TableKind::Weapon, 127, "Empty"),
        (TableKind::Shield, 224, "None"),
        (TableKind::Armor, 225, "None"),
        (TableKind::Helmet, 226, "None"),
        (TableKind::Gloves, 227, "None"),
    ];
    for (kind, id, name) in slots {
        let entry = ff4().placeholder(kind).unwrap();
        assert_eq!((entry.id, entry.kind, entry.placeholder), (id, kind, true), "{kind:?}");
        assert_eq!(entry.name(Language::En), Some(name), "{kind:?}");
        assert_eq!(ff4().entry(kind, id), Some(entry));
    }
    let mut marked: Vec<u32> = EQUIPMENT
        .iter()
        .flat_map(|kind| table(*kind).iter().filter(|entry| entry.placeholder).map(|entry| entry.id))
        .collect();
    marked.sort_unstable();
    assert_eq!(marked, [127, 224, 225, 226, 227]);
    for kind in [TableKind::Item, TableKind::KeyItem] {
        assert!(ff4().placeholder(kind).is_none(), "{kind:?}");
    }
}

#[test]
fn group_6_weapons_are_in_the_weapon_table() {
    // Without the group 6 reading the table would hold 74 entries and no content 521.
    assert_eq!(table(TableKind::Weapon).len(), 93);
    let last = ff4().entry(TableKind::Weapon, 521).unwrap();
    assert_eq!((last.kind, last.placeholder), (TableKind::Weapon, false));
    assert!(last.name_en.is_some() && last.name_fr.is_some());
    assert_eq!(table(TableKind::Weapon).iter().filter(|entry| entry.placeholder).count(), 1);
}

#[test]
fn the_ability_type_ids_match_the_document() {
    // `#master-data-per-game`, "Abilities": 40 values.
    let mut histogram = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        *histogram.entry(entry.ability.unwrap().type_id).or_insert(0_usize) += 1;
    }
    // Pairs of (type_id, rows), flat so the list stays short.
    let pairs: [usize; 80] = [
        1, 53, 2, 59, 3, 52, 4, 7, 5, 1, 6, 1, 7, 1, 8, 1, 9, 30, 10, 7, 11, 4, 12, 1, 13, 96, 14, 2, 15, 3, 16, 1, 17, 2, 18, 1, 19, 1, 20, 1, 21, 1, 22, 5,
        23, 1, 24, 1, 25, 1, 26, 1, 27, 1, 28, 2, 29, 2, 30, 1, 31, 1, 32, 1, 33, 1, 34, 8, 35, 89, 36, 2, 37, 1, 38, 1, 40, 1, 41, 1,
    ];
    let expected: BTreeMap<u32, usize> = pairs.chunks(2).map(|pair| (u32::try_from(pair[0]).unwrap(), pair[1])).collect();
    assert_eq!(expected.len(), 40);
    assert_eq!(histogram, expected);
    assert_eq!(histogram.values().sum::<usize>(), 446);
}

#[test]
fn the_ability_kinds_match_the_document() {
    let mut kinds = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        let info = entry.ability.unwrap();
        assert_eq!(kind_of(info.type_id), Some(info.kind), "ability {}", entry.id);
        *kinds.entry(format!("{:?}", info.kind)).or_insert(0_usize) += 1;
    }
    let expected: BTreeMap<String, usize> = [
        ("WhiteMagic", 53),
        ("BlackMagic", 59),
        ("Summon", 30),
        ("Ninjutsu", 7),
        ("Other", 4),
        ("Monster", 148),
        ("Command", 45),
        ("Internal", 100),
    ]
    .into_iter()
    .map(|(kind, count)| (kind.to_owned(), count))
    .collect();
    assert_eq!(kinds, expected);
}

#[test]
fn every_ability_has_level_0() {
    // FF I reads 1 to 8 for a spell; FF IV stores 0 on every row.
    for entry in table(TableKind::Ability) {
        assert_eq!(entry.ability.unwrap().level, 0, "ability {}", entry.id);
    }
}

#[test]
fn the_nameless_abilities_have_a_key_none() {
    // `#rows-without-a-name`: 139 rows with key None, none with a key in no file, none without a
    // content row. The tool refuses the last two causes in FF IV, so every nameless row is the first.
    let none = nameless(TableKind::Ability);
    assert_eq!(none.len(), 139);
    for entry in table(TableKind::Ability) {
        assert_eq!(entry.name_en.is_none(), entry.name_fr.is_none(), "ability {}", entry.id);
    }
    // The documented keyless rows a player can reach, and the 89 NonAct rows (type 35).
    for id in [76, 223, 224, 225, 278] {
        assert!(none.contains(&id), "ability {id}");
    }
    let non_act: Vec<u32> = table(TableKind::Ability)
        .iter()
        .filter(|entry| entry.ability.unwrap().type_id == 35)
        .map(|entry| entry.id)
        .collect();
    assert_eq!(non_act.len(), 89);
    assert!(non_act.iter().all(|id| none.contains(id)));
    // Named rows are the other 307.
    assert_eq!(table(TableKind::Ability).iter().filter(|entry| entry.name_en.is_some()).count(), 446 - 139);
}

#[test]
fn job_15_and_command_4_have_no_name() {
    assert_eq!(nameless(TableKind::Job), [15]);
    assert_eq!(nameless(TableKind::Command), [4]);
    // The entries exist: saves hold command 4 in `commandList`.
    assert!(ff4().entry(TableKind::Command, 4).is_some());
    assert!(ff4().entry(TableKind::Job, 15).is_some());
    for kind in CONTENT {
        assert!(nameless(kind).is_empty(), "{kind:?}");
    }
    for entry in table(TableKind::Job).iter().chain(table(TableKind::Command)) {
        assert_eq!(entry.name_en.is_none(), entry.name_fr.is_none(), "{:?} {}", entry.kind, entry.id);
    }
}

#[test]
fn every_ability_has_a_unique_content_id() {
    let mut seen = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        let content_id = entry.ability.unwrap().content_id.expect("FF IV has no contentless ability");
        assert_ne!(content_id, 0);
        assert_eq!(seen.insert(content_id, entry.id), None, "content {content_id}");
    }
    assert_eq!(seen.len(), 446);
    let (content_id, id) = seen.iter().next().map(|(content, id)| (*content, *id)).unwrap();
    let entry = ff4().entry(TableKind::Ability, id).unwrap();
    assert_eq!(ff4().ability_by_content(content_id), Some(entry));
    assert_eq!(ff4().ability_by_content(0), None);
}

#[test]
fn the_bounds_match_the_document() {
    let tables = ff4();
    assert_eq!(tables.bounds().count(), 31);
    let named = [
        ("MAX_GIL", 9_999_999),
        ("MAX_ITEM", 99),
        ("MAX_LV", 99),
        ("MAX_EXP", 9_999_999),
        ("MAX_HP", 9_999),
        ("MAX_MP", 999),
        // value1; the row's value2 of 24 is not carried (`#bounds-in-the-master-data`).
        ("MAX_GRADUAL_PETRIFICATION", 4),
    ];
    for (key, value) in named {
        assert_eq!(tables.bound(key), Some(value), "{key}");
    }
    assert_ne!(tables.bound("MAX_GRADUAL_PETRIFICATION"), Some(24));
    assert_eq!(tables.bound("MAX_MAGICAL_TIMES"), None);
    assert_eq!(tables.bound("MAX_MAGICAL_LEVEL"), None);
    assert!(tables.bounds().all(|(key, value)| key.starts_with("MAX_") && value >= 0));
}

#[test]
fn there_are_no_growth_tables() {
    // character_status.csv holds 14 rows, but growth waits for per-game research.
    assert_eq!(ff4().base_hp(1), None);
    assert_eq!(ff4().base_vitality(1), None);
    assert_eq!(ff4().exp_to_reach(1), None);
    assert_eq!(ff4().growth_step(1, 2), None);
}

#[test]
fn a_content_id_is_no_ability_id() {
    // Content 521 is a weapon, not an ability.
    assert!(ff4().entry(TableKind::Ability, 521).is_none());
    assert!(ff4().content(521).is_some());
}
