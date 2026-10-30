//! The committed FF V tables (build 20265087), after
//! `docs/formats/pixel-remaster.md#master-data-per-game`, `#content-types-5-to-9` and
//! `#rows-without-a-name`. Counts come from the document, not from the file.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;

use common::{game_tables, kind_rule::kind_of};
use neoffse_core::{Game, TableKind};
use neoffse_data::{AbilityKind, Entry, GameTables, Language, sources_for};

/// The seven content tables FF I has, in the order of the figures of the document.
const CONTENT: [TableKind; 7] = [
    TableKind::Item,
    TableKind::KeyItem,
    TableKind::Weapon,
    TableKind::Shield,
    TableKind::Armor,
    TableKind::Helmet,
    TableKind::Gloves,
];

/// The five equipment tables: `CONTENT` without the items.
const EQUIPMENT: [TableKind; 5] = [CONTENT[2], CONTENT[3], CONTENT[4], CONTENT[5], CONTENT[6]];

fn ff5() -> &'static GameTables {
    game_tables(Game::Ff5Pr)
}

fn table(kind: TableKind) -> &'static [Entry] {
    ff5().table(kind).unwrap()
}

fn ids(kind: TableKind) -> Vec<u32> {
    table(kind).iter().map(|entry| entry.id).collect()
}

fn nameless(kind: TableKind) -> Vec<u32> {
    table(kind).iter().filter(|entry| entry.name_en.is_none()).map(|entry| entry.id).collect()
}

#[test]
fn the_game_is_ff5_and_a_fresh_parse_equals_the_cached_tables() {
    let fresh = GameTables::parse(Game::Ff5Pr, &sources_for(Game::Ff5Pr).unwrap()).unwrap();
    assert_eq!(&fresh, ff5());
    assert_eq!(ff5().game(), Game::Ff5Pr);
}

#[test]
fn counts_per_table() {
    let content = [
        (TableKind::Item, 31),
        (TableKind::KeyItem, 16),
        (TableKind::Weapon, 108),
        (TableKind::Shield, 12),
        (TableKind::Armor, 27),
        (TableKind::Helmet, 22),
        // parts_group_id 5 holds 21 rings and no gloves, plus the empty slot, and reads as gloves.
        (TableKind::Gloves, 22),
        (TableKind::CommandContent, 236),
    ];
    for (kind, count) in content {
        assert_eq!(table(kind).len(), count, "{kind:?}");
    }
    assert_eq!(content.iter().map(|(_, count)| count).sum::<usize>(), 474);
    assert_eq!(table(TableKind::Ability).len(), 1004);
    assert_eq!(table(TableKind::Job).len(), 22);
    assert_eq!(table(TableKind::Command).len(), 238);
}

#[test]
fn only_the_tables_ff5_has_exist() {
    for kind in [TableKind::Monster, TableKind::Keyword, TableKind::MagicStone, TableKind::Map] {
        assert!(ff5().table(kind).is_none(), "{kind:?}");
        assert!(ff5().entry(kind, 1).is_none(), "{kind:?}");
    }
}

#[test]
fn ids_rise_inside_every_table() {
    let others = [TableKind::CommandContent, TableKind::Ability, TableKind::Job, TableKind::Command];
    for kind in CONTENT.into_iter().chain(others) {
        let list = ids(kind);
        assert!(list.windows(2).all(|pair| pair[0] < pair[1]), "{kind:?}");
    }
}

#[test]
fn documented_id_ranges() {
    // Inventory note, "Jobs and commands": job IDs 1-22.
    assert_eq!(ids(TableKind::Job), (1..=22).collect::<Vec<_>>());
    // Measured in content.csv, item.csv, weapon.csv, armor.csv, command.csv and ability.csv of
    // build 20265087, not in the committed files or a note. Content 1 is gil, not listed.
    let ranges = [
        (TableKind::Item, 2, 56, 31),
        (TableKind::KeyItem, 39, 58, 16),
        (TableKind::Weapon, 27, 173, 108),
        (TableKind::Shield, 174, 188, 12),
        (TableKind::Armor, 175, 240, 27),
        (TableKind::Helmet, 176, 212, 22),
        (TableKind::Gloves, 177, 266, 22),
        (TableKind::CommandContent, 874, 1656, 236),
        (TableKind::Command, 1, 252, 238),
        (TableKind::Ability, 1, 1175, 1004),
    ];
    for (kind, first, last, count) in ranges {
        let list = ids(kind);
        assert_eq!((list.first(), list.last(), list.len()), (Some(&first), Some(&last), count), "{kind:?}");
    }
}

#[test]
fn the_empty_slot_rows() {
    // `#master-data-per-game`, "Equipment groups".
    for (kind, id) in EQUIPMENT.into_iter().zip([59, 174, 175, 176, 177]) {
        let entry = ff5().placeholder(kind).unwrap();
        assert_eq!((entry.id, entry.kind, entry.placeholder), (id, kind, true), "{kind:?}");
        // The English text by which the inventory identified them.
        assert_eq!(
            entry.name(Language::En),
            Some(if kind == TableKind::Weapon { "Empty" } else { "None" }),
            "{kind:?}"
        );
        assert_eq!(ff5().entry(kind, id), Some(entry));
    }
    let mut marked: Vec<u32> = EQUIPMENT
        .iter()
        .flat_map(|kind| table(*kind).iter().filter(|entry| entry.placeholder).map(|entry| entry.id))
        .collect();
    marked.sort_unstable();
    assert_eq!(marked, [59, 174, 175, 176, 177]);
    for kind in [TableKind::Item, TableKind::KeyItem, TableKind::CommandContent] {
        assert!(ff5().placeholder(kind).is_none(), "{kind:?}");
        assert!(table(kind).iter().all(|entry| !entry.placeholder), "{kind:?}");
    }
}

#[test]
fn command_content_rows_are_keyed_by_content_id() {
    // `#content-types-5-to-9`: a type-7 row is read by its content ID, not by the command ID.
    let entry = ff5().entry(TableKind::CommandContent, 918).unwrap();
    assert_eq!((entry.kind, entry.ability), (TableKind::CommandContent, None));
    // Content 877 points to command 4; the command ID is no key of this table.
    assert!(ff5().entry(TableKind::CommandContent, 4).is_none());
    assert!(ff5().entry(TableKind::Command, 918).is_none());
    // The generic content lookup keeps its seven kinds.
    assert!(ff5().content(918).is_none());
    assert!(ff5().content(2).is_some());
}

#[test]
fn four_command_contents_have_no_name() {
    // `#rows-without-a-name` and `#content-types-5-to-9`: contents 879, 1030 and 1503 have key
    // None. Content 877 has key None too: its command, 4, has no name either.
    let none = nameless(TableKind::CommandContent);
    assert_eq!(none, [877, 879, 1030, 1503]);
    for id in &none {
        let entry = ff5().entry(TableKind::CommandContent, *id).unwrap();
        assert_eq!((entry.name(Language::En), entry.name(Language::Fr)), (None, None), "content {id}");
    }
    // The 32 levelled job abilities (918 to 949) and every other row keep their own key.
    assert_eq!(table(TableKind::CommandContent).iter().filter(|entry| entry.name_en.is_some()).count(), 232);
    for id in 918..=949 {
        let entry = ff5().entry(TableKind::CommandContent, id).unwrap();
        assert!(entry.name_en.is_some() && entry.name_fr.is_some(), "content {id}");
    }
    for kind in CONTENT {
        assert!(nameless(kind).is_empty(), "{kind:?}");
    }
}

#[test]
fn the_ability_type_ids_match_the_document() {
    // `#master-data-per-game`, "Abilities": 43 values.
    let mut histogram = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        *histogram.entry(entry.ability.unwrap().type_id).or_insert(0_usize) += 1;
    }
    // Pairs of (type_id, rows).
    let pairs: [usize; 86] = [
        1, 60, 2, 75, 3, 164, 4, 6, 5, 2, 6, 3, 7, 1, 8, 1, 9, 68, 10, 1, 11, 9, 12, 1, 13, 196, 15, 2, 16, 2, 19, 1, 21, 2, 25, 2, 26, 1, 27, 2, 28, 1, 31, 2,
        34, 10, 35, 187, 38, 1, 40, 1, 41, 1, 301, 1, 303, 2, 501, 39, 502, 19, 503, 1, 504, 64, 505, 60, 506, 1, 507, 1, 508, 1, 512, 1, 513, 1, 514, 6, 515,
        1, 516, 2, 609, 2,
    ];
    let expected: BTreeMap<u32, usize> = pairs.chunks(2).map(|pair| (u32::try_from(pair[0]).unwrap(), pair[1])).collect();
    assert_eq!(expected.len(), 43);
    assert_eq!(histogram, expected);
    assert_eq!(histogram.values().sum::<usize>(), 1004);
}

#[test]
fn the_ability_kinds_match_the_document() {
    let mut kinds = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        let info = entry.ability.unwrap();
        assert_eq!(kind_of(info.type_id), Some(info.kind), "ability {}", entry.id);
        *kinds.entry(format!("{:?}", info.kind)).or_insert(0_usize) += 1;
    }
    let names = "WhiteMagic BlackMagic Summon Ninjutsu Other TimeMagic MagicSword BlueMagic Monster Command Internal".split(' ');
    let counts = [60, 75, 68, 1, 9, 39, 19, 64, 360, 109, 200];
    let expected: BTreeMap<String, usize> = names.zip(counts).map(|(kind, count)| (kind.to_owned(), count)).collect();
    assert_eq!(kinds, expected);
}

#[test]
fn the_spell_levels_match_the_document() {
    // `#master-data-per-game`, "Abilities": 207 rows above level 0, a highest of 6 (`MAX_LV_PROG`
    // is 255 and `MAX_LV` 99).
    let raised: Vec<_> = table(TableKind::Ability).iter().filter(|entry| entry.ability.unwrap().level > 0).collect();
    assert_eq!(raised.len(), 207);
    assert_eq!(raised.iter().map(|entry| entry.ability.unwrap().level).max(), Some(6));
}

#[test]
fn the_three_causes_of_a_nameless_ability() {
    // `#rows-without-a-name`: 207 rows with key None, 2 with a key in no file (484, 686), 1 with
    // no content row (1090). The three sets are disjoint.
    let none = nameless(TableKind::Ability);
    assert_eq!(none.len(), 207 + 2 + 1);
    for entry in table(TableKind::Ability) {
        assert_eq!(entry.name_en.is_none(), entry.name_fr.is_none(), "ability {}", entry.id);
    }
    let contentless: Vec<u32> = table(TableKind::Ability)
        .iter()
        .filter(|entry| entry.ability.unwrap().content_id.is_none())
        .map(|entry| entry.id)
        .collect();
    assert_eq!(contentless, [1090]);
    // Enum `MagicFight` is 513, in the `command` list.
    let info = ff5().entry(TableKind::Ability, 1090).unwrap().ability.unwrap();
    assert_eq!((info.type_id, info.kind), (513, AbilityKind::Command));
    for id in [484, 686] {
        let entry = ff5().entry(TableKind::Ability, id).unwrap();
        assert!(entry.name_en.is_none() && entry.ability.unwrap().content_id.is_some(), "ability {id}");
    }
    let named = table(TableKind::Ability).iter().filter(|entry| entry.name_en.is_some()).count();
    assert_eq!(named, 1004 - 210);
    // Documented rows that qualify by type: 575 and 1165.
    assert!(none.contains(&575) && none.contains(&1165));
}

#[test]
fn command_4_has_no_name() {
    // Saves hold command 4 in `commandList`; the 237 others are named.
    assert_eq!(nameless(TableKind::Command), [4]);
    assert!(nameless(TableKind::Job).is_empty(), "jobs");
    for entry in table(TableKind::Job).iter().chain(table(TableKind::Command)) {
        assert_eq!(entry.name_en.is_none(), entry.name_fr.is_none(), "{:?} {}", entry.kind, entry.id);
    }
}

#[test]
fn every_other_ability_has_a_unique_content_id() {
    let mut seen = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        let Some(content_id) = entry.ability.unwrap().content_id else { continue };
        assert_ne!(content_id, 0);
        assert_eq!(seen.insert(content_id, entry.id), None, "content {content_id}");
    }
    assert_eq!(seen.len(), 1003);
    // Ability 1090 has no content ID, so no lookup by content ID returns it.
    for (content_id, id) in &seen {
        assert_eq!(ff5().ability_by_content(*content_id).map(|entry| entry.id), Some(*id));
    }
    assert!(!seen.values().any(|id| *id == 1090));
    assert_eq!(ff5().ability_by_content(0), None);
}

#[test]
fn the_bounds_match_the_document() {
    let tables = ff5();
    assert_eq!(tables.bounds().count(), 32);
    let named = [
        ("MAX_GIL", 9_999_999),
        ("MAX_ITEM", 99),
        ("MAX_LV", 99),
        ("MAX_EXP", 9_999_999),
        ("MAX_HP", 9_999),
        ("MAX_MP", 999),
        ("MAX_LV_PROG", 255),
    ];
    for (key, value) in named {
        assert_eq!(tables.bound(key), Some(value), "{key}");
    }
    for absent in ["MAX_MAGICAL_TIMES", "MAX_GRADUAL_PETRIFICATION", "MAX_MAGICAL_LEVEL"] {
        assert_eq!(tables.bound(absent), None, "{absent}");
    }
    assert!(tables.bounds().all(|(key, value)| key.starts_with("MAX_") && value >= 0));
}

#[test]
fn there_are_no_growth_tables() {
    // exp_table.csv holds 22 curves, but growth waits for per-game research.
    assert_eq!(ff5().base_hp(1), None);
    assert_eq!(ff5().base_vitality(1), None);
    assert_eq!(ff5().exp_to_reach(1), None);
    assert_eq!(ff5().growth_step(1, 2), None);
}
