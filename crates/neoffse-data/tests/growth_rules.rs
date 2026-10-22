//! Rules of `growth.toml`: one failing text per rule over the minimal sources.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::error::Error;

use common::minimal;
use neoffse_core::Game;
use neoffse_data::{DataError, GameTables, Sources};

fn parse(growth: &str) -> Result<GameTables, DataError> {
    GameTables::parse(Game::Ff1Pr, &Sources { growth, ..minimal() })
}

fn statuses(ids: &[u32]) -> String {
    let rows: Vec<String> = ids.iter().map(|id| format!("[[character_status]]\nid = {id}\nhp = 10\n\n")).collect();
    rows.concat()
}

fn levels(rows: &[(u32, u32)]) -> String {
    let rows: Vec<String> = rows
        .iter()
        .map(|(lv, total)| format!("[[exp_table]]\nlv = {lv}\ntotal_exp = {total}\n\n"))
        .collect();
    rows.concat()
}

/// A valid status array followed by `exp_table` rows.
fn with_levels(rows: &[(u32, u32)]) -> String {
    statuses(&[1, 2]) + &levels(rows)
}

fn toml_file(result: Result<GameTables, DataError>) -> &'static str {
    match result.unwrap_err() {
        DataError::Toml { file, .. } => file,
        other => panic!("expected a Toml error, got {other}"),
    }
}

fn id_order(result: Result<GameTables, DataError>) -> (&'static str, u32) {
    match result.unwrap_err() {
        DataError::IdOrder { file, id } => (file, id),
        other => panic!("expected an IdOrder error, got {other}"),
    }
}

fn rule(result: Result<GameTables, DataError>) -> (&'static str, String, &'static str) {
    match result.unwrap_err() {
        DataError::Rule { file, row, rule } => (file, row, rule),
        other => panic!("expected a Rule error, got {other}"),
    }
}

#[test]
fn the_minimal_growth_text_serves_its_rows() {
    let tables = parse(common::GROWTH).unwrap();
    assert_eq!(tables.base_hp(2), Some(20));
    assert_eq!(tables.base_hp(3), None);
    assert_eq!(tables.base_hp(4), Some(30));
    assert_eq!(tables.exp_to_reach(3), Some(12));
    assert_eq!(tables.exp_to_reach(5), None);
}

#[test]
fn a_malformed_growth_text_is_a_toml_error_naming_growth_toml() {
    let error = parse("[[character_status\nid = 1").unwrap_err();
    assert!(matches!(error, DataError::Toml { file: "growth.toml", .. }));
    assert!(error.source().is_some());
}

#[test]
fn an_unknown_growth_field_is_refused() {
    let text = common::GROWTH.replacen("hp = 10", "hp = 10\nsort_order = 1", 1);
    assert_eq!(toml_file(parse(&text)), "growth.toml");
    let text = common::GROWTH.replacen("total_exp = 5", "total_exp = 5\nsort_order = 1", 1);
    assert_eq!(toml_file(parse(&text)), "growth.toml");
}

#[test]
fn a_missing_growth_array_is_refused() {
    assert_eq!(toml_file(parse(&statuses(&[1]))), "growth.toml");
    assert_eq!(toml_file(parse(&levels(&[(1, 0)]))), "growth.toml");
    assert_eq!(toml_file(parse("")), "growth.toml");
}

#[test]
fn a_negative_hp_and_an_oversized_total_are_toml_errors() {
    let text = common::GROWTH.replacen("hp = 10", "hp = -1", 1);
    assert_eq!(toml_file(parse(&text)), "growth.toml");
    let text = common::GROWTH.replacen("total_exp = 40", "total_exp = 4294967296", 1);
    assert_eq!(toml_file(parse(&text)), "growth.toml");
}

#[test]
fn a_status_id_of_zero_a_repeat_or_a_descent_is_an_id_order_error() {
    let exp = levels(&[(1, 0), (2, 5)]);
    for (ids, bad) in [(&[0, 1][..], 0), (&[1, 1], 1), (&[2, 1], 1)] {
        let text = statuses(ids) + &exp;
        assert_eq!(id_order(parse(&text)), ("growth.toml", bad), "{ids:?}");
    }
}

#[test]
fn a_level_gap_is_refused() {
    let text = with_levels(&[(1, 0), (2, 5), (4, 40)]);
    assert_eq!(
        rule(parse(&text)),
        ("growth.toml", "lv 4".to_string(), "level does not follow the level before it")
    );
}

#[test]
fn a_first_level_of_two_is_refused() {
    let text = with_levels(&[(2, 5), (3, 12)]);
    assert_eq!(
        rule(parse(&text)),
        ("growth.toml", "lv 2".to_string(), "level does not follow the level before it")
    );
}

#[test]
fn a_repeated_level_is_an_id_order_error() {
    assert_eq!(id_order(parse(&with_levels(&[(1, 0), (2, 5), (2, 9)]))), ("growth.toml", 2));
    assert_eq!(id_order(parse(&with_levels(&[(0, 0), (1, 5)]))), ("growth.toml", 0));
}

#[test]
fn an_equal_or_falling_total_is_refused() {
    for rows in [[(1, 0), (2, 5), (3, 5)], [(1, 0), (2, 5), (3, 4)]] {
        assert_eq!(rule(parse(&with_levels(&rows))), ("growth.toml", "lv 3".to_string(), "total_exp does not rise"));
    }
    assert!(parse(&with_levels(&[(1, 7), (2, 8)])).is_ok(), "the first total is the tool's check");
}

#[test]
fn an_empty_array_is_refused() {
    let exp = levels(&[(1, 0)]);
    let text = format!("character_status = []\n{exp}");
    assert_eq!(rule(parse(&text)), ("growth.toml", "character_status".to_string(), "table has no row"));
    let text = format!("exp_table = []\n{}", statuses(&[1]));
    assert_eq!(rule(parse(&text)), ("growth.toml", "exp_table".to_string(), "table has no row"));
}

#[test]
fn the_display_text_of_a_growth_rule() {
    let error = parse(&with_levels(&[(1, 0), (2, 5), (4, 40)])).unwrap_err();
    assert_eq!(error.to_string(), "growth.toml: lv 4 breaks a rule: level does not follow the level before it");
}
