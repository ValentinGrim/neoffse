//! The embedded text, the games with data and the cache.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::path::PathBuf;

use common::ff1;
use neoffse_core::Game;
use neoffse_data::{GameTables, sources_for, tables_for};

fn assert_send_sync<T: Send + Sync>() {}

fn committed(file: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "data", "pr", "ff1", file].iter().collect();
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn the_embedded_text_is_the_committed_file() {
    let sources = sources_for(Game::Ff1Pr).unwrap();
    assert_eq!(sources.content, committed("content.toml"));
    assert_eq!(sources.abilities, committed("abilities.toml"));
    assert_eq!(sources.jobs, committed("jobs.toml"));
    assert_eq!(sources.commands, committed("commands.toml"));
    assert_eq!(sources.monsters, Some(committed("monsters.toml").as_str()));
    assert_eq!(sources.bounds, committed("bounds.toml"));
    assert_eq!(sources.growth, Some(committed("growth.toml").as_str()));
}

#[test]
fn only_ff1_has_data() {
    for game in Game::ALL.iter().copied().filter(|game| *game != Game::Ff1Pr) {
        assert!(sources_for(game).is_none(), "{game:?}");
        assert!(tables_for(game).is_none(), "{game:?}");
    }
    assert!(sources_for(Game::Ff1Pr).is_some());
}

#[test]
fn tables_for_returns_one_instance() {
    let first = tables_for(Game::Ff1Pr).unwrap();
    assert!(std::ptr::eq(first, tables_for(Game::Ff1Pr).unwrap()));
    let handles: Vec<_> = (0..2).map(|_| std::thread::spawn(|| tables_for(Game::Ff1Pr).unwrap())).collect();
    for handle in handles {
        assert!(std::ptr::eq(handle.join().unwrap(), first));
    }
    assert_send_sync::<GameTables>();
}

#[test]
fn game_reports_its_game() {
    assert_eq!(ff1().game(), Game::Ff1Pr);
}
