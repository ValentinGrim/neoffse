//! The embedded text, the games with data and the cache.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::path::PathBuf;

use common::{EMBEDDED, ff1, game_tables};
use neoffse_core::{Game, TableKind};
use neoffse_data::{GameTables, sources_for, tables_for};

fn assert_send_sync<T: Send + Sync>() {}

/// The committed `data/pr/<game>/<file>`; `None` when the file does not exist.
fn committed(game: Game, file: &str) -> Option<String> {
    let folder = game.id().trim_end_matches("-pr");
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "..", "..", "data", "pr", folder, file].iter().collect();
    path.exists().then(|| std::fs::read_to_string(path).unwrap())
}

fn required(game: Game, file: &str) -> String {
    committed(game, file).unwrap()
}

#[test]
fn the_embedded_text_is_the_committed_file() {
    for game in EMBEDDED.iter().copied() {
        let sources = sources_for(game).unwrap();
        assert_eq!(sources.content, required(game, "content.toml"), "{game:?}");
        assert_eq!(sources.abilities, required(game, "abilities.toml"), "{game:?}");
        assert_eq!(sources.jobs, required(game, "jobs.toml"), "{game:?}");
        assert_eq!(sources.commands, required(game, "commands.toml"), "{game:?}");
        assert_eq!(sources.bounds, required(game, "bounds.toml"), "{game:?}");
        // An optional file is embedded exactly when it is committed.
        assert_eq!(sources.monsters.map(str::to_owned), committed(game, "monsters.toml"), "{game:?}");
        assert_eq!(sources.growth.map(str::to_owned), committed(game, "growth.toml"), "{game:?}");
    }
}

#[test]
fn exactly_the_embedded_games_have_data() {
    for game in Game::ALL.iter().copied() {
        let has_data = EMBEDDED.contains(&game);
        assert_eq!(sources_for(game).is_some(), has_data, "{game:?}");
        assert_eq!(tables_for(game).is_some(), has_data, "{game:?}");
    }
}

#[test]
fn games_do_not_share_tables() {
    for (index, first) in EMBEDDED.iter().copied().enumerate() {
        for second in EMBEDDED.iter().copied().skip(index + 1) {
            assert!(!std::ptr::eq(game_tables(first), game_tables(second)), "{first:?} {second:?}");
            let size = |game| game_tables(game).table(TableKind::Ability).unwrap().len();
            assert_ne!(size(first), size(second), "{first:?} {second:?}");
        }
    }
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
