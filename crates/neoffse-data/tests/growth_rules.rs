//! Rules of `growth.toml`: one failing text per rule over the minimal sources.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::error::Error;

use common::{curve_text, id_order, parse_growth, rule, toml_file};
use neoffse_data::DataError;

fn statuses(ids: &[u32]) -> String {
    let rows: Vec<String> = ids
        .iter()
        .map(|id| format!("[[character_status]]\nid = {id}\nhp = 10\nvitality = 7\n\n"))
        .collect();
    rows.concat()
}

fn levels(totals: &[u32]) -> String {
    let values: Vec<String> = totals.iter().map(u32::to_string).collect();
    format!("[exp_table]\ntotal_exp = [{}]\n\n", values.join(", "))
}

/// A valid status array, the `exp_table` of `totals` and valid curve rows
/// that cover its levels.
fn with_levels(totals: &[u32]) -> String {
    let last = u32::try_from(totals.len()).unwrap();
    statuses(&[1, 2]) + &levels(totals) + &curve_text(&[1, 2], last)
}

#[test]
fn the_minimal_growth_text_serves_its_rows() {
    let tables = parse_growth(common::GROWTH).unwrap();
    assert_eq!(tables.base_hp(2), Some(20));
    assert_eq!(tables.base_hp(3), None);
    assert_eq!(tables.base_hp(4), Some(30));
    assert_eq!(tables.exp_to_reach(3), Some(12));
    assert_eq!(tables.exp_to_reach(5), None);
}

#[test]
fn a_malformed_growth_text_is_a_toml_error_naming_growth_toml() {
    let error = parse_growth("[[character_status\nid = 1").unwrap_err();
    assert!(matches!(error, DataError::Toml { file: "growth.toml", .. }));
    assert!(error.source().is_some());
}

#[test]
fn an_unknown_growth_field_is_refused() {
    let text = common::GROWTH.replacen("vitality = 7", "vitality = 7\nsort_order = 1", 1);
    assert_eq!(toml_file(parse_growth(&text)), "growth.toml");
    let text = common::GROWTH.replacen("total_exp = [0, 5, 12, 40]", "total_exp = [0, 5, 12, 40]\nsort_order = 1", 1);
    assert_eq!(toml_file(parse_growth(&text)), "growth.toml");
}

#[test]
fn a_missing_growth_array_is_refused() {
    let curve = curve_text(&[1], 4);
    assert_eq!(toml_file(parse_growth(&(statuses(&[1]) + &curve))), "growth.toml");
    assert_eq!(toml_file(parse_growth(&(levels(&[0]) + &curve))), "growth.toml");
    assert_eq!(toml_file(parse_growth("")), "growth.toml");
    let text = common::GROWTH.replacen("vitality = 7", "", 1);
    assert_eq!(toml_file(parse_growth(&text)), "growth.toml", "a status without vitality");
}

#[test]
fn a_negative_hp_and_an_oversized_total_are_toml_errors() {
    let text = common::GROWTH.replacen("hp = 10", "hp = -1", 1);
    assert_eq!(toml_file(parse_growth(&text)), "growth.toml");
    let text = common::GROWTH.replacen("12, 40]", "12, 4294967296]", 1);
    assert_eq!(toml_file(parse_growth(&text)), "growth.toml");
    let text = common::GROWTH.replacen("[0, 5, 12, 40]", "[0, 5, -12, 40]", 1);
    assert_eq!(toml_file(parse_growth(&text)), "growth.toml");
}

#[test]
fn a_status_id_of_zero_a_repeat_or_a_descent_is_an_id_order_error() {
    let tail = levels(&[0, 5]) + &curve_text(&[1], 2);
    for (ids, bad) in [(&[0, 1][..], 0), (&[1, 1], 1), (&[2, 1], 1)] {
        let text = statuses(ids) + &tail;
        assert_eq!(id_order(parse_growth(&text)), ("growth.toml", bad), "{ids:?}");
    }
}

#[test]
fn an_equal_or_falling_total_is_refused() {
    for totals in [[0, 5, 5], [0, 5, 4]] {
        assert_eq!(
            rule(parse_growth(&with_levels(&totals))),
            ("growth.toml", "exp_table lv 3".to_string(), "total_exp does not rise")
        );
    }
    assert_eq!(
        rule(parse_growth(&with_levels(&[3, 2]))),
        ("growth.toml", "exp_table lv 2".to_string(), "total_exp does not rise")
    );
    assert!(parse_growth(&with_levels(&[7, 8])).is_ok(), "the first total is the tool's check");
}

#[test]
fn an_empty_array_is_refused() {
    let tail = levels(&[0]) + &curve_text(&[1], 1);
    let text = format!("character_status = []\n{tail}");
    assert_eq!(rule(parse_growth(&text)), ("growth.toml", "character_status".to_string(), "table has no row"));
    let text = statuses(&[1]) + &levels(&[]) + &curve_text(&[1], 1);
    assert_eq!(rule(parse_growth(&text)), ("growth.toml", "exp_table".to_string(), "table has no row"));
}

#[test]
fn the_display_text_of_a_growth_rule() {
    let error = parse_growth(&with_levels(&[0, 5, 5])).unwrap_err();
    assert_eq!(error.to_string(), "growth.toml: exp_table lv 3 breaks a rule: total_exp does not rise");
}
