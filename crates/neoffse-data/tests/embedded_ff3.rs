//! The committed FF III tables (build 20264899), after
//! `docs/formats/pixel-remaster.md#master-data-per-game` and
//! `#rows-without-a-name`. Counts come from the document, not from the file.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;

use common::{game_tables, kind_rule::kind_of};
use neoffse_core::{Game, TableKind};
use neoffse_data::{AbilityKind, Entry, GameTables, Language, sources_for};

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

fn ff3() -> &'static GameTables {
    game_tables(Game::Ff3Pr)
}

fn table(kind: TableKind) -> &'static [Entry] {
    ff3().table(kind).unwrap()
}

fn ids(kind: TableKind) -> Vec<u32> {
    table(kind).iter().map(|entry| entry.id).collect()
}

fn nameless(kind: TableKind) -> Vec<u32> {
    table(kind).iter().filter(|entry| entry.name_en.is_none()).map(|entry| entry.id).collect()
}

#[test]
fn game_is_ff3() {
    assert_eq!(ff3().game(), Game::Ff3Pr);
}

#[test]
fn a_fresh_parse_equals_the_cached_tables() {
    let fresh = GameTables::parse(Game::Ff3Pr, &sources_for(Game::Ff3Pr).unwrap()).unwrap();
    assert_eq!(&fresh, ff3());
}

#[test]
fn counts_per_table() {
    let content = [
        (TableKind::Item, 92),
        (TableKind::KeyItem, 12),
        (TableKind::Weapon, 85),
        (TableKind::Shield, 11),
        (TableKind::Armor, 26),
        (TableKind::Helmet, 17),
        // parts_group_id 5 holds 5 rings and 8 gloves, plus the empty slot, and reads as gloves.
        (TableKind::Gloves, 14),
    ];
    for (kind, count) in content {
        assert_eq!(table(kind).len(), count, "{kind:?}");
    }
    assert_eq!(content.iter().map(|(_, count)| count).sum::<usize>(), 257);
    assert_eq!(table(TableKind::Ability).len(), 209);
    assert_eq!(table(TableKind::Job).len(), 22);
    assert_eq!(table(TableKind::Command).len(), 28);
}

#[test]
fn only_the_tables_ff3_has_exist() {
    for kind in [
        TableKind::Monster,
        TableKind::Keyword,
        TableKind::MagicStone,
        TableKind::CommandContent,
        TableKind::Map,
    ] {
        assert!(ff3().table(kind).is_none(), "{kind:?}");
        assert!(ff3().entry(kind, 1).is_none(), "{kind:?}");
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
    // Inventory note, "Jobs and commands": job IDs 1-22, command IDs 1-28.
    assert_eq!(ids(TableKind::Job), (1..=22).collect::<Vec<_>>());
    assert_eq!(ids(TableKind::Command), (1..=28).collect::<Vec<_>>());
    // The ranges below are measured in content.csv and ability.csv of build 20264899, not in the
    // committed files and not in a note. Content 1 is gil, which the file does not list.
    let first_last = |kind| {
        let list = ids(kind);
        (list.first().copied(), list.last().copied())
    };
    assert_eq!(first_last(TableKind::Item), (Some(2), Some(333)));
    assert_eq!(first_last(TableKind::KeyItem), (Some(38), Some(49)));
    assert_eq!(first_last(TableKind::Weapon), (Some(50), Some(134)));
    let worn: Vec<u32> = EQUIPMENT[1..].iter().flat_map(|kind| ids(*kind)).collect();
    assert_eq!((worn.iter().min(), worn.iter().max()), (Some(&135), Some(&202)));
    let all: Vec<u32> = CONTENT.iter().flat_map(|kind| ids(*kind)).collect();
    assert_eq!((all.iter().min(), all.iter().max()), (Some(&2), Some(&333)));
    assert_eq!(first_last(TableKind::Ability), (Some(1), Some(212)));
}

#[test]
fn the_empty_slot_rows() {
    // `#master-data-per-game`, "Equipment groups": one row per group 1 to 5.
    let slots = [
        (TableKind::Weapon, 50, "Empty"),
        (TableKind::Shield, 135, "None"),
        (TableKind::Armor, 136, "None"),
        (TableKind::Helmet, 137, "None"),
        (TableKind::Gloves, 138, "None"),
    ];
    for (kind, id, name) in slots {
        let entry = ff3().placeholder(kind).unwrap();
        assert_eq!((entry.id, entry.kind, entry.placeholder), (id, kind, true), "{kind:?}");
        assert_eq!(entry.name(Language::En), Some(name), "{kind:?}");
        assert_eq!(ff3().entry(kind, id), Some(entry));
    }
    let mut marked: Vec<u32> = EQUIPMENT
        .iter()
        .flat_map(|kind| table(*kind).iter().filter(|entry| entry.placeholder).map(|entry| entry.id))
        .collect();
    marked.sort_unstable();
    assert_eq!(marked, [50, 135, 136, 137, 138]);
    for kind in [TableKind::Item, TableKind::KeyItem] {
        assert!(ff3().placeholder(kind).is_none(), "{kind:?}");
    }
}

#[test]
fn the_ability_type_ids_match_the_document() {
    // `#master-data-per-game`, "Abilities": 23 values.
    let mut histogram = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        *histogram.entry(entry.ability.unwrap().type_id).or_insert(0_usize) += 1;
    }
    let pairs: [usize; 46] = [
        1, 36, 2, 56, 4, 25, 5, 1, 6, 2, 7, 1, 8, 1, 9, 34, 11, 4, 12, 1, 13, 32, 15, 1, 16, 1, 20, 1, 21, 1, 28, 1, 31, 1, 301, 1, 302, 1, 303, 1, 304, 1,
        305, 1, 306, 5,
    ];
    let expected: BTreeMap<u32, usize> = pairs.chunks(2).map(|pair| (u32::try_from(pair[0]).unwrap(), pair[1])).collect();
    assert_eq!(expected.len(), 23);
    assert_eq!(histogram, expected);
    assert_eq!(histogram.values().sum::<usize>(), 209);
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
        ("WhiteMagic", 36),
        ("BlackMagic", 56),
        ("Summon", 34),
        ("Other", 4),
        ("Monster", 32),
        ("Command", 46),
        ("Internal", 1),
    ]
    .into_iter()
    .map(|(kind, count)| (kind.to_owned(), count))
    .collect();
    assert_eq!(kinds, expected);
}

#[test]
fn the_spell_levels_match_the_document() {
    // `#master-data-per-game`, "Abilities": 117 rows above level 0, a highest of 8, on types 1, 2 and 9.
    let raised: Vec<_> = table(TableKind::Ability).iter().filter(|entry| entry.ability.unwrap().level > 0).collect();
    assert_eq!(raised.len(), 117);
    assert_eq!(raised.iter().map(|entry| entry.ability.unwrap().level).max(), Some(8));
    let mut types: Vec<u32> = raised.iter().map(|entry| entry.ability.unwrap().type_id).collect();
    types.sort_unstable();
    types.dedup();
    assert_eq!(types, [1, 2, 9]);
}

#[test]
fn the_nameless_abilities_have_a_key_none() {
    // `#rows-without-a-name`: 4 rows with key None, none with a key in no file, none without a
    // content row. IDs 134 and 136 are in the document (134 qualifies by type, 136 sits in saves);
    // 91 and 138 are measured in ability.csv of build 20264899, with the other two causes absent.
    assert_eq!(nameless(TableKind::Ability), [91, 134, 136, 138]);
    for entry in table(TableKind::Ability) {
        assert_eq!(entry.name_en.is_none(), entry.name_fr.is_none(), "ability {}", entry.id);
    }
    // The document names the four by `AbilityType`: Summon, TwoSwordStyle, JumpDown, KarateAccumulate.
    let types: Vec<u32> = [91, 134, 136, 138]
        .iter()
        .map(|id| ff3().entry(TableKind::Ability, *id).unwrap().ability.unwrap().type_id)
        .collect();
    assert_eq!(types, [16, 9, 12, 306]);
    assert_eq!(table(TableKind::Ability).iter().filter(|entry| entry.name_en.is_some()).count(), 209 - 4);
}

#[test]
fn ability_136_is_the_two_sword_style_of_every_character() {
    // Kinds note [M]: 8 characters in 2 slots hold it, with offset 250 over the content row.
    let entry = ff3().entry(TableKind::Ability, 136).unwrap();
    let info = entry.ability.unwrap();
    assert_eq!((info.type_id, info.kind, info.content_id), (12, AbilityKind::Internal, Some(386)));
    assert_eq!((entry.name_en.as_deref(), entry.name_fr.as_deref()), (None, None));
    assert_eq!(ff3().ability_by_content(386), Some(entry));
}

#[test]
fn commands_10_14_and_22_have_no_name() {
    // `#rows-without-a-name`: their keys sit in no message file. A strict commands table
    // would have stopped the export; the other 25 commands are named.
    assert_eq!(nameless(TableKind::Command), [10, 14, 22]);
    for id in [10, 14, 22] {
        let entry = ff3().entry(TableKind::Command, id).unwrap();
        assert_eq!(
            (entry.kind, entry.name(Language::En), entry.name(Language::Fr)),
            (TableKind::Command, None, None)
        );
    }
    assert_eq!(table(TableKind::Command).iter().filter(|entry| entry.name_en.is_some()).count(), 25);
    assert!(ff3().entry(TableKind::Command, 1).unwrap().name(Language::En).is_some());
    assert!(nameless(TableKind::Job).is_empty(), "jobs");
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
        let content_id = entry.ability.unwrap().content_id.expect("FF III has no contentless ability");
        assert_ne!(content_id, 0);
        assert_eq!(seen.insert(content_id, entry.id), None, "content {content_id}");
    }
    assert_eq!(seen.len(), 209);
    assert_eq!(ff3().ability_by_content(0), None);
}

#[test]
fn the_bounds_match_the_document() {
    let tables = ff3();
    assert_eq!(tables.bounds().count(), 27);
    let named = [
        ("MAX_GIL", 9_999_999),
        ("MAX_ITEM", 99),
        ("MAX_LV", 99),
        ("MAX_EXP", 9_999_999),
        ("MAX_HP", 9_999),
        // FF III has spell charges and no MP.
        ("MAX_MAGICAL_TIMES", 99),
        // value1; the row's value2 of 24 is not carried (`#bounds-in-the-master-data`).
        ("MAX_GRADUAL_PETRIFICATION", 3),
    ];
    for (key, value) in named {
        assert_eq!(tables.bound(key), Some(value), "{key}");
    }
    assert_ne!(tables.bound("MAX_GRADUAL_PETRIFICATION"), Some(24));
    assert_eq!(tables.bound("MAX_MP"), None);
    assert_eq!(tables.bound("MAX_MAGICAL_LEVEL"), None);
    assert!(tables.bounds().all(|(key, value)| key.starts_with("MAX_") && value >= 0));
}

#[test]
fn there_are_no_growth_tables() {
    // character_status.csv holds 4 rows, but growth waits for per-game research.
    assert_eq!(ff3().base_hp(1), None);
    assert_eq!(ff3().base_vitality(1), None);
    assert_eq!(ff3().exp_to_reach(1), None);
    assert_eq!(ff3().growth_step(1, 2), None);
}

#[test]
fn a_content_id_is_no_ability_id() {
    // 386 is the content row of ability 136, which the content table does not list.
    assert!(ff3().entry(TableKind::Ability, 386).is_none());
    assert!(ff3().content(386).is_none());
}
