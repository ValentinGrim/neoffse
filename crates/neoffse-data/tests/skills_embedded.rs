//! The committed FF II `skills.toml` (build 20251646), after
//! `docs/formats/pixel-remaster.md#people-and-party-in-ff-ii`, `#skill-levels-in-ff-ii`
//! and `#spells-in-ff-ii`. The figures come from the document, not from the file.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{EMBEDDED, game_tables};
use neoffse_core::Game;
use neoffse_data::{GameTables, tables_for};

fn ff2() -> &'static GameTables {
    game_tables(Game::Ff2Pr)
}

/// The `(chain, level)` pair of an ability id.
fn step(ability_id: u32) -> Option<(u32, u32)> {
    ff2().spell_step(ability_id).map(|step| (step.chain, step.level))
}

#[test]
fn the_base_hp_and_mp_follow_the_document() {
    let expected = [
        (1, 30, 10),
        (2, 20, 10),
        (3, 40, 10),
        (4, 175, 103),
        (5, 235, 5),
        (6, 128, 15),
        (7, 248, 35),
        (8, 549, 5),
        (9, 1098, 5),
        (10, 30, 5),
    ];
    for (id, hp, mp) in expected {
        let base = ff2().base_stats(id).unwrap();
        assert_eq!((base.hp, base.mp), (hp, mp), "status {id}");
    }
}

#[test]
fn a_status_without_a_row_has_no_base() {
    assert_eq!(ff2().base_stats(0), None);
    assert_eq!(ff2().base_stats(11), None);
}

#[test]
fn the_first_status_reads_every_column() {
    let base = ff2().base_stats(1).unwrap();
    assert_eq!(
        (base.power, base.vitality, base.agility, base.intelligence, base.spirit, base.magic),
        (10, 10, 10, 10, 10, 5)
    );
}

#[test]
fn the_skill_totals_follow_the_curve() {
    let expected = [0, 20, 45, 70, 100, 135, 175, 220, 270, 330, 400, 480, 570, 660, 760, 860];
    for (index, total) in expected.iter().enumerate() {
        let level = u32::try_from(index).unwrap() + 1;
        assert_eq!(ff2().skill_total(level), Some(*total), "level {level}");
    }
}

#[test]
fn a_skill_level_outside_the_curve_has_no_total() {
    assert_eq!(ff2().skill_total(0), None);
    assert_eq!(ff2().skill_total(17), None);
}

#[test]
fn the_document_examples_give_their_levels() {
    assert_eq!(step(335), Some((20, 11)));
    assert_eq!(step(12), Some((0, 8)));
    assert_eq!(step(330), Some((20, 6)));
    assert_eq!(step(5), Some((0, 1)));
    assert_eq!(step(20), Some((0, 16)));
    assert_eq!(step(21), Some((1, 1)));
    assert_eq!(step(644), Some((39, 16)));
}

#[test]
fn an_id_next_to_the_chains_has_no_step() {
    for id in [0, 4, 645, 711] {
        assert_eq!(step(id), None, "ability {id}");
    }
}

#[test]
fn a_chain_or_level_outside_the_chains_has_no_ability() {
    assert_eq!(ff2().spell_ability(39, 17), None);
    assert_eq!(ff2().spell_ability(40, 1), None);
    assert_eq!(ff2().spell_ability(0, 0), None);
    assert_eq!(ff2().spell_ability(0, 1), Some(5));
    assert_eq!(ff2().spell_ability(39, 16), Some(644));
}

#[test]
fn spell_ability_inverts_spell_step_on_all_640_ids() {
    for id in 5..=644 {
        let (chain, level) = step(id).unwrap();
        assert_eq!(ff2().spell_ability(chain, level), Some(id), "ability {id}");
    }
}

#[test]
fn ff2_has_forty_chains() {
    assert_eq!(ff2().spell_chains(), 40);
}

#[test]
fn the_other_games_answer_none_or_zero() {
    for game in EMBEDDED.iter().filter(|game| **game != Game::Ff2Pr) {
        let tables = tables_for(*game).unwrap();
        assert_eq!(tables.base_stats(1), None, "{game:?}");
        assert_eq!(tables.skill_total(1), None, "{game:?}");
        assert_eq!(tables.spell_step(5), None, "{game:?}");
        assert_eq!(tables.spell_ability(0, 1), None, "{game:?}");
        assert_eq!(tables.spell_chains(), 0, "{game:?}");
    }
}
