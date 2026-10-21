//! The tables the crate ships, embedded at build time.

use std::sync::OnceLock;

use neoffse_core::Game;

use crate::error::DataError;
use crate::tables::{GameTables, Sources};

// The path leaves the crate folder: the crate is `publish = false`. A
// publication to crates.io has to copy the tables into the crate first
// (docs/architecture.md#open-points).
const FF1: Sources<'static> = Sources {
    content: include_str!("../../../data/pr/ff1/content.toml"),
    abilities: include_str!("../../../data/pr/ff1/abilities.toml"),
    jobs: include_str!("../../../data/pr/ff1/jobs.toml"),
    commands: include_str!("../../../data/pr/ff1/commands.toml"),
    monsters: include_str!("../../../data/pr/ff1/monsters.toml"),
    bounds: include_str!("../../../data/pr/ff1/bounds.toml"),
};

static FF1_TABLES: OnceLock<GameTables> = OnceLock::new();

/// The embedded text of a game. `None` for a game with no data: only
/// `Game::Ff1Pr` has data today.
#[must_use]
pub fn sources_for(game: Game) -> Option<Sources<'static>> {
    match game {
        Game::Ff1Pr => Some(FF1),
        _ => None,
    }
}

/// The parsed embedded tables of a game, parsed once and shared by every
/// caller. `None` for a game with no data.
///
/// # Panics
///
/// Panics when the embedded text is invalid. The text is a constant of the
/// build and the tests of this crate parse it, so a release never panics
/// here. [`GameTables::parse`] is the fallible route for other text.
#[must_use]
pub fn tables_for(game: Game) -> Option<&'static GameTables> {
    match game {
        Game::Ff1Pr => Some(FF1_TABLES.get_or_init(|| load(Game::Ff1Pr, &FF1))),
        _ => None,
    }
}

fn load(game: Game, sources: &Sources<'_>) -> GameTables {
    match GameTables::parse(game, sources) {
        Ok(tables) => tables,
        Err(error) => panic!("{}", describe(&error)),
    }
}

/// The error and its source, from `Display` only.
fn describe(error: &DataError) -> String {
    match std::error::Error::source(error) {
        Some(source) => format!("{error}: {source}"),
        None => error.to_string(),
    }
}
