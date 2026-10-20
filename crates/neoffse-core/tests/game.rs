//! The game list: ids, order and lookup.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::collections::{BTreeSet, HashSet};

use neoffse_core::Game;

#[test]
fn ids_follow_the_game_id_rule() {
    // The rule of `DataDir::game_backups_dir`, restated: neoffse-core depends
    // on no workspace crate.
    for game in Game::ALL {
        let id = game.id();
        assert_ne!(id, "");
        assert!(id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'), "{id}");
    }
}

#[test]
fn all_lists_six_games_in_title_order() {
    assert_eq!(Game::ALL.len(), 6);
    let expected = ["ff1-pr", "ff2-pr", "ff3-pr", "ff4-pr", "ff5-pr", "ff6-pr"];
    for (index, game) in Game::ALL.iter().enumerate() {
        let number = u8::try_from(index + 1).unwrap();
        assert_eq!(game.number(), number);
        assert_eq!(game.id(), format!("ff{number}-pr"));
        assert_eq!(game.id(), expected[index]);
    }
    let unique: HashSet<&str> = Game::ALL.iter().map(|game| game.id()).collect();
    assert_eq!(unique.len(), 6);
}

#[test]
fn from_id_inverts_id() {
    for game in Game::ALL {
        assert_eq!(Game::from_id(game.id()), Some(*game));
    }
}

#[test]
fn from_id_rejects_near_misses() {
    for id in ["", "FF1-PR", "ff1", "ff1-pr ", " ff1-pr", "ff1-pr\n", "ff7-pr", "ff0-pr", "ff1-pr-"] {
        assert_eq!(Game::from_id(id), None, "{id:?}");
    }
}

#[test]
fn a_game_is_a_map_key() {
    let mut games: Vec<Game> = Game::ALL.to_vec();
    games.push(Game::Ff1Pr);
    let ordered: BTreeSet<Game> = games.iter().copied().collect();
    let hashed: HashSet<Game> = games.iter().copied().collect();
    assert_eq!(ordered.len(), 6);
    assert_eq!(hashed.len(), 6);
    assert_eq!(ordered.first(), Some(&Game::Ff1Pr));
}
