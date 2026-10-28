//! Rules of the growth rows in `growth.toml`: one failing text per rule over
//! the minimal sources.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{group_text, id_order, job_text, parse_growth, rule, toml_file};
use neoffse_data::{DataError, GameTables, GrowthStep};

/// Statuses and an `exp_table` that ends at level 4.
fn head() -> String {
    "[[character_status]]\nid = 1\nhp = 10\nvitality = 7\n\n[exp_table]\ntotal_exp = [0, 5, 12, 40]\n\n".to_string()
}

fn expect_rule(result: Result<GameTables, DataError>, row: &str, message: &str) {
    assert_eq!(rule(result), ("growth.toml", row.to_string(), message));
}

#[test]
fn the_minimal_growth_text_serves_its_rows() {
    let tables = parse_growth(common::GROWTH).unwrap();
    assert_eq!(tables.base_vitality(2), Some(8));
    assert_eq!(tables.base_vitality(3), None);
    let step = tables.growth_step(5, 3).unwrap();
    assert!(step.strong_hp);
    assert_eq!(step.stats, [2, 1, 0, 0, 0, 0, 1]);
    assert_eq!((step.accuracy_rate, step.magic_evasion_rate), (5, 6));
    assert_eq!(step.charges, [1, 2, 3, 4, 5, 6, 7, 8]);
    let step = tables.growth_step(1, 4).unwrap();
    assert!(!step.strong_hp);
    assert_eq!(step.stats, [1, 2, 0, 0, 0, 0, 1]);
    assert_eq!((step.accuracy_rate, step.magic_evasion_rate), (4, 5));
    assert_eq!(tables.growth_step(3, 2), None, "no job 3");
    assert_eq!(tables.growth_step(1, 5), None, "above the last level");
    assert_eq!(tables.growth_step(1, 1), None, "level 1 has no row");
    assert_eq!(tables.growth_step(1, 0), None);
    assert_eq!(tables.growth_step(1, u32::MAX), None);
}

#[test]
fn a_job_reads_the_group_it_names_not_the_group_of_its_own_id() {
    let tables = parse_growth(common::GROWTH).unwrap();
    let by_job: Vec<Option<GrowthStep>> = [1, 2, 5].iter().map(|job| tables.growth_step(*job, 2)).collect();
    assert_eq!(by_job.iter().map(|step| step.unwrap().accuracy_rate).collect::<Vec<_>>(), [4, 5, 5]);
}

#[test]
fn an_empty_job_table_is_refused() {
    let text = "intermediate_growth_curve = []\n".to_string() + &head() + &group_text(1, 4);
    expect_rule(parse_growth(&text), "intermediate_growth_curve", "table has no row");
}

#[test]
fn an_empty_curve_table_is_refused() {
    let text = "growth_curve = []\n".to_string() + &head() + &job_text(&[(1, 1)]);
    expect_rule(parse_growth(&text), "growth_curve", "table has no row");
}

#[test]
fn a_repeated_or_descending_job_is_an_id_order_error() {
    let groups = group_text(1, 4);
    for (jobs, bad) in [(&[(0, 1)][..], 0), (&[(1, 1), (1, 1)], 1), (&[(2, 1), (1, 1)], 1)] {
        let text = head() + &job_text(jobs) + &groups;
        assert_eq!(id_order(parse_growth(&text)), ("growth.toml", bad), "{jobs:?}");
    }
}

#[test]
fn a_group_that_does_not_rise_is_refused() {
    let jobs = job_text(&[(1, 1)]);
    for (order, bad) in [(&[2, 1][..], 1), (&[1, 1], 1), (&[1, 2, 1], 1), (&[0], 0)] {
        let groups: String = order.iter().map(|g| group_text(*g, 4)).collect();
        let text = head() + &jobs + &groups;
        expect_rule(parse_growth(&text), &format!("growth_curve group {bad}"), "group does not rise");
    }
}

#[test]
fn a_group_that_does_not_start_at_level_two_is_refused() {
    for first in [0, 1, 3] {
        let groups = group_text(1, 4).replacen("first_lv = 2", &format!("first_lv = {first}"), 1);
        let text = head() + &job_text(&[(1, 1)]) + &groups;
        expect_rule(parse_growth(&text), "growth_curve group 1", "group does not start at level 2");
    }
}

#[test]
fn a_column_that_does_not_cover_the_exp_levels_is_refused() {
    let jobs = job_text(&[(1, 1)]);
    let short = group_text(1, 3);
    let text = head() + &jobs + &short;
    expect_rule(parse_growth(&text), "growth_curve group 1", "hp_value1 does not cover the levels of exp_table");
    let long = group_text(1, 5);
    let text = head() + &jobs + &long;
    expect_rule(parse_growth(&text), "growth_curve group 1", "hp_value1 does not cover the levels of exp_table");
    let one = group_text(1, 4).replacen("magical_times8 = [8, 8, 8]", "magical_times8 = [8, 8]", 1);
    let text = head() + &jobs + &one;
    expect_rule(
        parse_growth(&text),
        "growth_curve group 1",
        "magical_times8 does not cover the levels of exp_table",
    );
    let one = group_text(1, 4).replacen("luck = [1, 1, 1]", "luck = []", 1);
    let text = head() + &jobs + &one;
    expect_rule(parse_growth(&text), "growth_curve group 1", "luck does not cover the levels of exp_table");
}

#[test]
fn hp_value1_of_two_is_refused() {
    let groups = group_text(1, 4).replacen("hp_value1 = [0, 1, 0]", "hp_value1 = [0, 2, 0]", 1);
    let text = head() + &job_text(&[(1, 1)]) + &groups;
    expect_rule(parse_growth(&text), "growth_curve group 1", "hp_value1 is neither 0 nor 1");
}

#[test]
fn a_job_with_an_unknown_group_is_refused() {
    let text = head() + &job_text(&[(1, 3)]) + &group_text(1, 4) + &group_text(2, 4);
    expect_rule(parse_growth(&text), "intermediate_growth_curve job 1", "growth group has no growth_curve row");
}

#[test]
fn a_missing_curve_array_an_unknown_field_and_a_negative_value_are_toml_errors() {
    let no_jobs = head() + &group_text(1, 4);
    assert_eq!(toml_file(parse_growth(&no_jobs)), "growth.toml");
    let no_groups = head() + &job_text(&[(1, 1)]);
    assert_eq!(toml_file(parse_growth(&no_groups)), "growth.toml");
    let no_column = common::GROWTH.replacen("luck = [1, 1, 1]\n", "", 1);
    assert_eq!(toml_file(parse_growth(&no_column)), "growth.toml");
    let unknown = common::GROWTH.replacen("first_lv = 2", "first_lv = 2\nsort_order = 1", 1);
    assert_eq!(toml_file(parse_growth(&unknown)), "growth.toml");
    let unknown = common::GROWTH.replacen("job_id = 1", "job_id = 1\nsort_order = 1", 1);
    assert_eq!(toml_file(parse_growth(&unknown)), "growth.toml");
    let negative = common::GROWTH.replacen("accuracy_rate = [4, 4, 4]", "accuracy_rate = [4, -1, 4]", 1);
    assert_eq!(toml_file(parse_growth(&negative)), "growth.toml");
}

#[test]
fn the_display_text_of_a_curve_rule() {
    let groups = group_text(2, 4).replacen("hp_value1 = [0, 1, 0]", "hp_value1 = [0, 2, 0]", 1);
    let text = head() + &job_text(&[(1, 2)]) + &groups;
    assert_eq!(
        parse_growth(&text).unwrap_err().to_string(),
        "growth.toml: growth_curve group 2 breaks a rule: hp_value1 is neither 0 nor 1"
    );
}
