//! Rules of `skills.toml`: one failing text per rule over the minimal sources.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::error::Error;

use common::{BOUNDS, id_order, minimal, rule, toml_file};
use neoffse_core::Game;
use neoffse_data::{DataError, GameTables, Sources};

/// Abilities 1 to 4 with content ids 101 to 104; `bare` has no content id.
fn abilities(bare: Option<u32>) -> String {
    let rows: Vec<String> = (1..=4)
        .map(|id| {
            let content = if bare == Some(id) {
                String::new()
            } else {
                format!("content_id = {}\n", 100 + id)
            };
            format!("[[ability]]\nid = {id}\n{content}type_id = 1\nkind = \"white_magic\"\nlevel = 1\nname_en = \"A\"\nname_fr = \"B\"\n\n")
        })
        .collect();
    rows.concat()
}

fn statuses(ids: &[u32]) -> String {
    let rows: Vec<String> = ids
        .iter()
        .map(|id| format!("[[status]]\nid = {id}\nhp = 10\nmp = 5\npower = 1\nvitality = 2\nagility = 3\nintelligence = 4\nspirit = 5\nmagic = 6\n\n"))
        .collect();
    rows.concat()
}

fn curve(totals: &[u32]) -> String {
    let values: Vec<String> = totals.iter().map(u32::to_string).collect();
    format!("[skill_curve]\ntotal_exp = [{}]\n\n", values.join(", "))
}

fn chains(first: &[u32], length: u32) -> String {
    let values: Vec<String> = first.iter().map(u32::to_string).collect();
    format!("[spell_chains]\nfirst = [{}]\nlength = {length}\n", values.join(", "))
}

/// A valid text: two statuses, a two-level curve and two chains of two abilities.
fn valid() -> String {
    statuses(&[1, 2]) + &curve(&[0, 5]) + &chains(&[1, 3], 2)
}

fn parse_with(skills: &str, abilities: &str, bounds: &str) -> Result<GameTables, DataError> {
    GameTables::parse(
        Game::Ff2Pr,
        &Sources {
            skills: Some(skills),
            abilities,
            bounds,
            ..minimal()
        },
    )
}

fn parse(skills: &str) -> Result<GameTables, DataError> {
    parse_with(skills, &abilities(None), BOUNDS)
}

fn broken(text: &str) -> (&'static str, String, &'static str) {
    rule(parse(text))
}

#[test]
fn the_valid_text_serves_its_rows() {
    let tables = parse(&valid()).unwrap();
    assert_eq!(tables.base_stats(2).map(|base| base.hp), Some(10));
    assert_eq!(tables.skill_total(2), Some(5));
    assert_eq!(tables.skill_total(3), None);
    assert_eq!(tables.spell_chains(), 2);
    let step = tables.spell_step(4).unwrap();
    assert_eq!((step.chain, step.level), (1, 2));
    assert_eq!(tables.spell_ability(1, 2), Some(4));
}

#[test]
fn a_malformed_text_is_a_toml_error_naming_skills_toml() {
    let error = parse("[[status\nid = 1").unwrap_err();
    assert!(matches!(error, DataError::Toml { file: "skills.toml", .. }));
    assert!(error.source().is_some());
}

#[test]
fn an_unknown_field_is_refused() {
    let text = valid().replacen("hp = 10", "hp = 10\nluck = 1", 1);
    assert_eq!(toml_file(parse(&text)), "skills.toml");
    let text = valid().replacen("length = 2", "length = 2\nwidth = 1", 1);
    assert_eq!(toml_file(parse(&text)), "skills.toml");
}

#[test]
fn a_missing_table_is_refused() {
    assert_eq!(toml_file(parse(&(curve(&[0, 5]) + &chains(&[1], 1)))), "skills.toml");
    assert_eq!(toml_file(parse(&(statuses(&[1]) + &chains(&[1], 1)))), "skills.toml");
    assert_eq!(toml_file(parse(&(statuses(&[1]) + &curve(&[0, 5])))), "skills.toml");
}

#[test]
fn an_empty_status_table_is_refused() {
    let text = "status = []\n".to_string() + &curve(&[0, 5]) + &chains(&[1], 1);
    assert_eq!(broken(&text), ("skills.toml", "status".to_string(), "table has no row"));
}

#[test]
fn a_first_status_id_other_than_1_is_refused() {
    for first in [0, 2] {
        let text = statuses(&[first, 3]) + &curve(&[0, 5]) + &chains(&[1], 1);
        assert_eq!(broken(&text), ("skills.toml", format!("status id {first}"), "first status id is not 1"));
    }
}

#[test]
fn status_ids_that_do_not_rise_are_refused() {
    let text = statuses(&[1, 1]) + &curve(&[0, 5]) + &chains(&[1], 1);
    assert_eq!(id_order(parse(&text)), ("skills.toml", 1));
}

#[test]
fn a_curve_starting_at_1_is_refused() {
    let text = statuses(&[1]) + &curve(&[1, 5, 12]) + &chains(&[1], 1);
    assert_eq!(broken(&text), ("skills.toml", "skill_curve lv 1".to_string(), "total_exp of level 1 is not 0"));
}

#[test]
fn an_empty_curve_is_refused() {
    let text = statuses(&[1]) + &curve(&[]) + &chains(&[1], 1);
    assert_eq!(broken(&text).1, "skill_curve lv 1");
}

#[test]
fn a_curve_that_stops_rising_is_refused() {
    let text = statuses(&[1]) + &curve(&[0, 5, 5, 40]) + &chains(&[1], 1);
    assert_eq!(broken(&text), ("skills.toml", "skill_curve lv 3".to_string(), "total_exp does not rise"));
}

#[test]
fn a_curve_of_15_values_against_the_level_caps_is_refused() {
    let totals: Vec<u32> = (0..15).map(|level| level * 20).collect();
    let text = statuses(&[1]) + &curve(&totals) + &chains(&[1], 1);
    let bounds = BOUNDS.to_string() + "MAX_MAGICAL_LEVEL = 16\nMAX_WEAPON_LEVEL = 16\n";
    let error = rule(parse_with(&text, &abilities(None), &bounds));
    assert_eq!(error, ("skills.toml", "skill_curve".to_string(), "row count differs from the level caps"));
    let bounds = BOUNDS.to_string() + "MAX_WEAPON_LEVEL = 16\n";
    assert_eq!(rule(parse_with(&text, &abilities(None), &bounds)).1, "skill_curve");
}

#[test]
fn a_curve_matching_the_level_caps_parses() {
    let bounds = BOUNDS.to_string() + "MAX_MAGICAL_LEVEL = 2\nMAX_WEAPON_LEVEL = 2\n";
    assert!(parse_with(&valid(), &abilities(None), &bounds).is_ok());
}

#[test]
fn an_empty_chain_list_is_refused() {
    let text = statuses(&[1]) + &curve(&[0, 5]) + &chains(&[], 1);
    assert_eq!(broken(&text), ("skills.toml", "spell_chains".to_string(), "table has no row"));
}

#[test]
fn a_chain_length_of_0_is_refused() {
    let text = statuses(&[1]) + &curve(&[0, 5]) + &chains(&[1, 3], 0);
    assert_eq!(broken(&text), ("skills.toml", "spell_chains".to_string(), "length is below 1"));
}

#[test]
fn a_chain_length_that_differs_from_the_curve_is_refused() {
    let expected = ("skills.toml", "spell_chains".to_string(), "chain length differs from the skill curve");
    for (totals, length) in [(&[0, 5, 12][..], 2), (&[0, 5][..], 3)] {
        let text = statuses(&[1]) + &curve(totals) + &chains(&[1, 4], length);
        assert_eq!(broken(&text), expected, "{} levels, length {length}", totals.len());
    }
}

#[test]
fn two_chains_that_overlap_are_refused() {
    let text = statuses(&[1]) + &curve(&[0, 5]) + &chains(&[1, 2], 2);
    assert_eq!(
        broken(&text),
        ("skills.toml", "spell_chains chain 1".to_string(), "chain starts inside the chain before it")
    );
}

#[test]
fn chains_that_do_not_rise_are_refused() {
    let text = statuses(&[1]) + &curve(&[0, 5]) + &chains(&[3, 1], 2);
    assert_eq!(broken(&text).2, "chain starts inside the chain before it");
}

#[test]
fn a_chain_id_that_is_no_ability_is_refused() {
    let text = statuses(&[1]) + &curve(&[0, 5]) + &chains(&[1, 4], 2);
    assert_eq!(
        broken(&text),
        ("skills.toml", "spell_chains chain 1 id 5".to_string(), "id is no ability with a content id")
    );
}

#[test]
fn an_ability_without_a_content_id_is_refused() {
    let error = rule(parse_with(&valid(), &abilities(Some(4)), BOUNDS));
    assert_eq!(
        error,
        ("skills.toml", "spell_chains chain 1 id 4".to_string(), "id is no ability with a content id")
    );
}

#[test]
fn the_display_text_of_a_chain_rule() {
    let text = statuses(&[1]) + &curve(&[0, 5]) + &chains(&[1, 2], 2);
    assert_eq!(
        parse(&text).unwrap_err().to_string(),
        "skills.toml: spell_chains chain 1 breaks a rule: chain starts inside the chain before it"
    );
}

#[test]
fn without_skills_the_accessors_answer_none() {
    let tables = GameTables::parse(Game::Ff2Pr, &Sources { skills: None, ..minimal() }).unwrap();
    assert_eq!(tables.base_stats(1), None);
    assert_eq!(tables.spell_chains(), 0);
}

#[test]
fn an_empty_text_is_not_none() {
    assert_eq!(toml_file(parse("")), "skills.toml");
}
