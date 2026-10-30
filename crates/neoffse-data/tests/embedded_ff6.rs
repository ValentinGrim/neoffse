//! The committed FF VI tables (build 20265119), after
//! `docs/formats/pixel-remaster.md#master-data-per-game`, `#content-types-5-to-9` and
//! `#rows-without-a-name`. Counts come from the document, not from the file.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;

use common::{game_tables, kind_rule::kind_of};
use neoffse_core::{Game, TableKind};
use neoffse_data::{Entry, GameTables, Language, sources_for};

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

fn ff6() -> &'static GameTables {
    game_tables(Game::Ff6Pr)
}

fn table(kind: TableKind) -> &'static [Entry] {
    ff6().table(kind).unwrap()
}

fn ids(kind: TableKind) -> Vec<u32> {
    table(kind).iter().map(|entry| entry.id).collect()
}

fn nameless(kind: TableKind) -> Vec<u32> {
    table(kind).iter().filter(|entry| entry.name_en.is_none()).map(|entry| entry.id).collect()
}

#[test]
fn the_game_is_ff6_and_a_fresh_parse_equals_the_cached_tables() {
    let fresh = GameTables::parse(Game::Ff6Pr, &sources_for(Game::Ff6Pr).unwrap()).unwrap();
    assert_eq!(&fresh, ff6());
    assert_eq!(ff6().game(), Game::Ff6Pr);
}

#[test]
fn counts_per_table() {
    let content = [
        (TableKind::Item, 37),
        (TableKind::KeyItem, 13),
        (TableKind::Weapon, 91),
        (TableKind::Shield, 16),
        (TableKind::Armor, 32),
        (TableKind::Helmet, 28),
        // parts_group_id 5 holds 50 rings and 5 gloves, plus the empty slot, and reads as gloves.
        (TableKind::Gloves, 56),
        (TableKind::CommandContent, 33),
        (TableKind::MagicStone, 27),
    ];
    for (kind, count) in content {
        assert_eq!(table(kind).len(), count, "{kind:?}");
    }
    assert_eq!(content.iter().map(|(_, count)| count).sum::<usize>(), 333);
    assert_eq!(table(TableKind::Ability).len(), 1631);
    assert_eq!(table(TableKind::Job).len(), 22);
    assert_eq!(table(TableKind::Command).len(), 696);
}

#[test]
fn only_the_tables_ff6_has_exist() {
    for kind in [TableKind::Monster, TableKind::Keyword, TableKind::Map] {
        assert!(ff6().table(kind).is_none(), "{kind:?}");
        assert!(ff6().entry(kind, 1).is_none(), "{kind:?}");
    }
}

#[test]
fn ids_rise_inside_every_table() {
    let others = [
        TableKind::CommandContent,
        TableKind::MagicStone,
        TableKind::Ability,
        TableKind::Job,
        TableKind::Command,
    ];
    for kind in CONTENT.into_iter().chain(others) {
        let list = ids(kind);
        assert!(list.windows(2).all(|pair| pair[0] < pair[1]), "{kind:?}");
    }
}

#[test]
fn documented_id_ranges() {
    // Inventory note, "Jobs and commands": job IDs 1-22. `#content-types-5-to-9`: espers are
    // contents 62 to 88.
    assert_eq!(ids(TableKind::Job), (1..=22).collect::<Vec<_>>());
    assert_eq!(ids(TableKind::MagicStone), (62..=88).collect::<Vec<_>>());
    // Not in a note: measured in the bundle (build 20265119) through the tool's output, and for
    // CommandContent, Command, Ability and Job also in content.csv, command.csv, ability.csv
    // and job.csv. Content 1 is gil, not listed.
    let measured = [
        (TableKind::Item, 2, 38, 37),
        (TableKind::KeyItem, 39, 58, 13),
        (TableKind::Weapon, 93, 183, 91),
        (TableKind::Shield, 197, 215, 16),
        (TableKind::Armor, 198, 274, 32),
        (TableKind::Helmet, 199, 242, 28),
        (TableKind::Gloves, 200, 329, 56),
        (TableKind::CommandContent, 1286, 1521, 33),
        (TableKind::Command, 1, 765, 696),
        (TableKind::Ability, 1, 5917, 1631),
    ];
    for (kind, first, last, count) in measured {
        let list = ids(kind);
        assert_eq!((list.first(), list.last(), list.len()), (Some(&first), Some(&last), count), "{kind:?}");
    }
}

#[test]
fn the_empty_slot_rows() {
    // `#master-data-per-game`, "Equipment groups".
    for (kind, id) in EQUIPMENT.into_iter().zip([93, 197, 198, 199, 200]) {
        let entry = ff6().placeholder(kind).unwrap();
        assert_eq!((entry.id, entry.kind, entry.placeholder), (id, kind, true), "{kind:?}");
        // The English text by which the inventory identified them.
        assert_eq!(
            entry.name(Language::En),
            Some(if kind == TableKind::Weapon { "Empty" } else { "None" }),
            "{kind:?}"
        );
        assert_eq!(ff6().entry(kind, id), Some(entry));
    }
    let mut marked: Vec<u32> = EQUIPMENT
        .iter()
        .flat_map(|kind| table(*kind).iter().filter(|entry| entry.placeholder).map(|entry| entry.id))
        .collect();
    marked.sort_unstable();
    assert_eq!(marked, [93, 197, 198, 199, 200]);
    for kind in [TableKind::Item, TableKind::KeyItem, TableKind::CommandContent, TableKind::MagicStone] {
        assert!(ff6().placeholder(kind).is_none(), "{kind:?}");
        assert!(table(kind).iter().all(|entry| !entry.placeholder), "{kind:?}");
    }
}

#[test]
fn espers_are_magic_stones_read_by_entry() {
    // `#content-types-5-to-9`: 27 espers, all with `MSG_MAGIC_NAME_*` keys, so all named.
    assert_eq!(nameless(TableKind::MagicStone), Vec::<u32>::new());
    let entry = ff6().entry(TableKind::MagicStone, 62).unwrap();
    assert_eq!((entry.kind, entry.ability), (TableKind::MagicStone, None));
    // The generic content lookup keeps its seven kinds.
    assert!(ff6().content(62).is_none());
    assert!(ff6().content(2).is_some());
}

#[test]
fn command_content_rows_are_keyed_by_content_id() {
    // `#content-types-5-to-9`: a type-7 row is read by its content ID, not by the command ID.
    let entry = ff6().entry(TableKind::CommandContent, 1286).unwrap();
    assert_eq!((entry.kind, entry.ability), (TableKind::CommandContent, None));
    assert!(ff6().entry(TableKind::CommandContent, 4).is_none());
    assert!(ff6().entry(TableKind::Command, 1286).is_none());
    assert!(ff6().content(1286).is_none());
}

#[test]
fn one_command_content_has_no_name() {
    // `#rows-without-a-name`: content 1289 has key None and points to command 4, which has no
    // name either. The 32 other type-7 rows are named.
    assert_eq!(nameless(TableKind::CommandContent), [1289]);
    let entry = ff6().entry(TableKind::CommandContent, 1289).unwrap();
    assert_eq!((entry.name(Language::En), entry.name(Language::Fr)), (None, None));
    assert_eq!(table(TableKind::CommandContent).iter().filter(|entry| entry.name_en.is_some()).count(), 32);
    for kind in CONTENT {
        assert!(nameless(kind).is_empty(), "{kind:?}");
    }
}

#[test]
fn the_ability_type_ids_match_the_document() {
    // `#master-data-per-game`, "Abilities": 48 values.
    let mut histogram = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        *histogram.entry(entry.ability.unwrap().type_id).or_insert(0_usize) += 1;
    }
    // Pairs of (type_id, rows).
    let pairs: [usize; 96] = [
        1, 72, 2, 115, 3, 458, 4, 37, 5, 1, 6, 1, 7, 1, 8, 1, 9, 29, 12, 1, 13, 116, 15, 2, 16, 3, 34, 99, 35, 287, 40, 1, 41, 1, 504, 29, 601, 15, 602, 9,
        603, 8, 604, 8, 605, 1, 606, 1, 607, 1, 608, 1, 609, 1, 610, 1, 611, 1, 612, 1, 613, 8, 614, 1, 615, 1, 616, 1, 617, 1, 618, 1, 619, 1, 620, 13, 621,
        2, 622, 3, 623, 3, 625, 252, 627, 1, 628, 2, 629, 13, 637, 24, 638, 1, 639, 1,
    ];
    let expected: BTreeMap<u32, usize> = pairs.chunks(2).map(|pair| (u32::try_from(pair[0]).unwrap(), pair[1])).collect();
    assert_eq!(expected.len(), 48);
    assert_eq!(histogram, expected);
    assert_eq!(histogram.values().sum::<usize>(), 1631);
}

#[test]
fn the_ability_kinds_match_the_document() {
    let mut kinds = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        let info = entry.ability.unwrap();
        assert_eq!(kind_of(info.type_id), Some(info.kind), "ability {}", entry.id);
        *kinds.entry(format!("{:?}", info.kind)).or_insert(0_usize) += 1;
    }
    let names = "WhiteMagic BlackMagic Summon BlueMagic Hissatsu SpecialAbility Dance Madou Rampage Monster Command Internal".split(' ');
    let counts = [72, 115, 29, 29, 15, 9, 8, 8, 252, 574, 113, 407];
    let expected: BTreeMap<String, usize> = names.zip(counts).map(|(kind, count)| (kind.to_owned(), count)).collect();
    assert_eq!(kinds, expected);
}

#[test]
fn every_ability_has_level_0() {
    // `#master-data-per-game`, "Abilities": `ability_lv` is 0 on every row, spells included.
    assert!(table(TableKind::Ability).iter().all(|entry| entry.ability.unwrap().level == 0));
}

#[test]
fn the_bounds_match_the_document() {
    let tables = ff6();
    assert_eq!(tables.bounds().count(), 27);
    let named = [
        ("MAX_GIL", 9_999_999),
        ("MAX_ITEM", 99),
        ("MAX_LV", 99),
        ("MAX_EXP", 15_000_000),
        ("MAX_HP", 9_999),
        ("MAX_MP", 999),
        ("MAX_STRNGTH", 128),
        ("MAX_STEPS", 9_999_999),
    ];
    for (key, value) in named {
        assert_eq!(tables.bound(key), Some(value), "{key}");
    }
    // Absent in FF VI, and a non-MAX_ key whose ID 353 repeats in game_constant_int.csv.
    for absent in ["MAX_MAGICAL_TIMES", "MAX_GRADUAL_PETRIFICATION", "MAX_LV_PROG", "CHARACTER_MOG_ASSET_POSTURE"] {
        assert_eq!(tables.bound(absent), None, "{absent}");
    }
    assert!(tables.bounds().all(|(key, value)| key.starts_with("MAX_") && value >= 0));
}

#[test]
fn there_are_no_growth_tables() {
    assert_eq!(ff6().base_hp(1), None);
    assert_eq!(ff6().base_vitality(1), None);
    assert_eq!(ff6().exp_to_reach(1), None);
    assert_eq!(ff6().growth_step(1, 2), None);
}
