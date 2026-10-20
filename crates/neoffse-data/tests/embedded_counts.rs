//! Sizes and ordering of the committed FF I tables (build 20249397). A
//! regenerated table that changes a count fails these tests on purpose.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::HashSet;

use common::ff1;
use neoffse_core::{Game, TableKind};
use neoffse_data::{AbilityKind, GameTables, sources_for, tables_for};

const CONTENT_KINDS: [TableKind; 7] = [
    TableKind::Item,
    TableKind::KeyItem,
    TableKind::Weapon,
    TableKind::Shield,
    TableKind::Armor,
    TableKind::Helmet,
    TableKind::Gloves,
];

const ALL_KINDS: [TableKind; 11] = [
    TableKind::Item,
    TableKind::KeyItem,
    TableKind::Weapon,
    TableKind::Shield,
    TableKind::Armor,
    TableKind::Helmet,
    TableKind::Gloves,
    TableKind::Ability,
    TableKind::Job,
    TableKind::Command,
    TableKind::Monster,
];

fn len(kind: TableKind) -> usize {
    ff1().table(kind).unwrap().len()
}

#[test]
fn counts_per_file() {
    assert_eq!(len(TableKind::Ability), 96);
    assert_eq!(len(TableKind::Job), 12);
    assert_eq!(len(TableKind::Command), 6);
    assert_eq!(len(TableKind::Monster), 128);
    assert_eq!(ff1().bounds().count(), 27);
    let content: Vec<usize> = CONTENT_KINDS.iter().map(|kind| len(*kind)).collect();
    assert_eq!(content, [19, 17, 41, 10, 17, 8, 9]);
    assert_eq!(content.iter().sum::<usize>(), 121);
}

#[test]
fn every_content_kind_maps_to_a_table() {
    let tables = GameTables::parse(Game::Ff1Pr, &sources_for(Game::Ff1Pr).unwrap()).unwrap();
    let total: usize = CONTENT_KINDS.iter().map(|kind| tables.table(*kind).unwrap().len()).sum();
    assert_eq!(total, 121);
}

#[test]
fn the_map_table_is_absent() {
    assert!(ff1().table(TableKind::Map).is_none());
    assert!(ff1().entry(TableKind::Map, 1).is_none());
}

#[test]
fn ability_kinds() {
    let count = |wanted: AbilityKind| {
        ff1()
            .table(TableKind::Ability)
            .unwrap()
            .iter()
            .filter(|entry| entry.ability.unwrap().kind == wanted)
            .count()
    };
    assert_eq!(count(AbilityKind::WhiteMagic), 33);
    assert_eq!(count(AbilityKind::BlackMagic), 32);
    assert_eq!(count(AbilityKind::Monster), 26);
    assert_eq!(count(AbilityKind::Command), 5);
}

#[test]
fn ids_rise_in_every_table() {
    for kind in ALL_KINDS {
        let ids: Vec<u32> = ff1().table(kind).unwrap().iter().map(|entry| entry.id).collect();
        assert!(ids[0] >= 1, "{kind:?}");
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]), "{kind:?}");
        assert!(ff1().table(kind).unwrap().iter().all(|entry| entry.kind == kind));
    }
    let mut seen = HashSet::new();
    for kind in CONTENT_KINDS {
        for entry in ff1().table(kind).unwrap() {
            assert!(seen.insert(entry.id), "content id {} repeats", entry.id);
        }
    }
    assert_eq!(seen.len(), 121);
}

#[test]
fn a_fresh_parse_equals_the_cached_tables() {
    let fresh = GameTables::parse(Game::Ff1Pr, &sources_for(Game::Ff1Pr).unwrap()).unwrap();
    assert_eq!(&fresh, tables_for(Game::Ff1Pr).unwrap());
}
