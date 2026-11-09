//! The row count of `exp_table` against `MAX_LV` in `bounds.toml`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{BOUNDS, BOUNDS_NO_LEVEL, minimal, rule};
use neoffse_core::Game;
use neoffse_data::{DataError, GameTables, Sources};

/// The minimal sources (four `exp_table` rows) with `MAX_LV` set to `cap`.
fn parse_with_cap(cap: i64) -> Result<GameTables, DataError> {
    let bounds = BOUNDS.replace("MAX_LV = 4", &format!("MAX_LV = {cap}"));
    GameTables::parse(Game::Ff1Pr, &Sources { bounds: &bounds, ..minimal() })
}

fn cap_rule() -> (&'static str, String, &'static str) {
    ("growth.toml", "exp_table".to_string(), "row count differs from MAX_LV")
}

#[test]
fn a_row_count_equal_to_max_lv_parses() {
    let tables = GameTables::parse(Game::Ff1Pr, &minimal()).unwrap();
    assert_eq!(tables.bound("MAX_LV"), Some(4));
    assert_eq!(tables.exp_to_reach(4), Some(40));
}

#[test]
fn fewer_rows_than_max_lv_are_refused() {
    assert_eq!(rule(parse_with_cap(5)), cap_rule());
}

#[test]
fn more_rows_than_max_lv_are_refused() {
    assert_eq!(rule(parse_with_cap(3)), cap_rule());
}

#[test]
fn a_max_lv_of_zero_is_refused() {
    assert_eq!(rule(parse_with_cap(0)), cap_rule());
}

#[test]
fn without_max_lv_the_count_is_free() {
    let sources = Sources {
        bounds: BOUNDS_NO_LEVEL,
        ..minimal()
    };
    let tables = GameTables::parse(Game::Ff1Pr, &sources).unwrap();
    assert_eq!(tables.bound("MAX_LV"), None);
    assert_eq!(tables.exp_to_reach(4), Some(40));
}

#[test]
fn the_display_text_of_the_cap_rule() {
    let error = parse_with_cap(5).unwrap_err();
    assert_eq!(error.to_string(), "growth.toml: exp_table breaks a rule: row count differs from MAX_LV");
}
