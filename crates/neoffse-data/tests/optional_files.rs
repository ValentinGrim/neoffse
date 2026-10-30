//! `monsters` and `growth` are optional inputs: `None` means the game ships no such table.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{ff1, minimal, toml_file};
use neoffse_core::{Game, TableKind};
use neoffse_data::{GameTables, Sources};

fn parse(sources: &Sources<'_>) -> GameTables {
    GameTables::parse(Game::Ff1Pr, sources).unwrap()
}

#[test]
fn a_game_without_growth_serves_every_other_table() {
    let with = parse(&minimal());
    assert!(with.base_hp(1).is_some() && with.growth_step(1, 2).is_some());
    let tables = parse(&Sources { growth: None, ..minimal() });
    assert_eq!(tables.base_hp(1), None);
    assert_eq!(tables.base_vitality(1), None);
    assert_eq!(tables.exp_to_reach(1), None);
    assert_eq!(tables.growth_step(1, 2), None);
    assert!(tables.entry(TableKind::Item, 5).is_some());
    assert!(tables.table(TableKind::Monster).is_some());
    assert_eq!(tables.bound("MAX_LV"), Some(4));
}

#[test]
fn a_game_without_monsters_has_no_monster_table() {
    let tables = parse(&Sources { monsters: None, ..minimal() });
    assert!(tables.table(TableKind::Monster).is_none());
    assert!(tables.entry(TableKind::Monster, 1).is_none());
    assert!(tables.entry(TableKind::Item, 5).is_some());
    assert!(tables.table(TableKind::Job).is_some());
    assert!(tables.table(TableKind::Command).is_some());
    assert!(tables.table(TableKind::Ability).is_some());
    assert_eq!(tables.exp_to_reach(2), Some(5));
}

#[test]
fn the_cap_check_needs_a_growth_table() {
    let bounds = common::BOUNDS.replace("MAX_LV = 4", "MAX_LV = 99");
    let tables = parse(&Sources {
        bounds: &bounds,
        growth: None,
        ..minimal()
    });
    assert_eq!(tables.bound("MAX_LV"), Some(99));
}

#[test]
fn an_empty_text_is_not_none() {
    let growth = GameTables::parse(Game::Ff1Pr, &Sources { growth: Some(""), ..minimal() });
    assert_eq!(toml_file(growth), "growth.toml");
    let monsters = GameTables::parse(
        Game::Ff1Pr,
        &Sources {
            monsters: Some(""),
            ..minimal()
        },
    );
    assert_eq!(toml_file(monsters), "monsters.toml");
}

#[test]
fn the_ff1_tables_still_carry_both_files() {
    assert_eq!(ff1().table(TableKind::Monster).unwrap().len(), 128);
    assert_eq!(ff1().exp_to_reach(99), Some(1_471_455));
}
