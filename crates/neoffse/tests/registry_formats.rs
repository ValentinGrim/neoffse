//! `Registry::from_formats` and `Registry::new`: which games a registry holds.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::StandIn;
use neoffse::Registry;
use neoffse::core::{Game, SaveFormat};
use neoffse::data::Language;

/// One entry per game that `Registry::new` registers.
const SHIPPED: [Game; 1] = [Game::Ff1Pr];

fn debug_of(games: &[Game]) -> String {
    let ids: Vec<String> = games.iter().map(|game| format!("{:?}", game.id())).collect();
    format!("Registry {{ games: [{}] }}", ids.join(", "))
}

#[test]
fn from_formats_lists_games_in_all_order_whatever_the_order_given() {
    let registry = Registry::from_formats(vec![StandIn::new(Game::Ff3Pr).arc(), StandIn::new(Game::Ff1Pr).arc()]);
    assert_eq!(registry.games(), vec![Game::Ff1Pr, Game::Ff3Pr]);
    assert_eq!(registry.format(Game::Ff3Pr).map(SaveFormat::game), Some(Game::Ff3Pr));
    assert!(registry.format(Game::Ff2Pr).is_none());
    assert_eq!(format!("{registry:?}"), r#"Registry { games: ["ff1-pr", "ff3-pr"] }"#);
}

#[test]
fn an_empty_registry_has_no_game() {
    let registry = Registry::from_formats(Vec::new());
    assert_eq!(registry.games(), Vec::<Game>::new());
    assert_eq!(format!("{registry:?}"), "Registry { games: [] }");
}

#[test]
#[should_panic(expected = "two formats for ff3-pr")]
fn two_formats_for_one_game_panic() {
    let _ = Registry::from_formats(vec![StandIn::new(Game::Ff3Pr).arc(), StandIn::new(Game::Ff3Pr).arc()]);
}

#[test]
fn the_shipped_registry_lists_the_shipped_games() {
    let registry = Registry::new(Language::En);
    assert_eq!(registry.games(), SHIPPED);
    assert_eq!(format!("{registry:?}"), debug_of(&SHIPPED));
}

#[test]
fn a_shipped_format_answers_for_its_game_and_no_other_does() {
    let registry = Registry::new(Language::En);
    for game in Game::ALL {
        let format = registry.format(*game);
        if SHIPPED.contains(game) {
            assert_eq!(format.map(SaveFormat::game), Some(*game));
        } else {
            assert!(format.is_none());
        }
    }
}
