//! The tables the crate ships, embedded at build time.

use std::sync::OnceLock;

use neoffse_core::Game;

use crate::error::DataError;
use crate::tables::{GameTables, Sources};

/// The text of one game and its parse, made on first use.
struct Embedded {
    sources: Sources<'static>,
    tables: OnceLock<GameTables>,
}

// The paths leave the crate folder: the crate is `publish = false`. A
// publication to crates.io has to copy the tables into the crate first
// (docs/architecture.md#open-points).
static FF1: Embedded = Embedded {
    sources: Sources {
        content: include_str!("../../../data/pr/ff1/content.toml"),
        abilities: include_str!("../../../data/pr/ff1/abilities.toml"),
        jobs: include_str!("../../../data/pr/ff1/jobs.toml"),
        commands: include_str!("../../../data/pr/ff1/commands.toml"),
        monsters: Some(include_str!("../../../data/pr/ff1/monsters.toml")),
        bounds: include_str!("../../../data/pr/ff1/bounds.toml"),
        growth: Some(include_str!("../../../data/pr/ff1/growth.toml")),
    },
    tables: OnceLock::new(),
};

static FF2: Embedded = Embedded {
    sources: Sources {
        content: include_str!("../../../data/pr/ff2/content.toml"),
        abilities: include_str!("../../../data/pr/ff2/abilities.toml"),
        jobs: include_str!("../../../data/pr/ff2/jobs.toml"),
        commands: include_str!("../../../data/pr/ff2/commands.toml"),
        monsters: None,
        bounds: include_str!("../../../data/pr/ff2/bounds.toml"),
        growth: None,
    },
    tables: OnceLock::new(),
};

fn embedded(game: Game) -> Option<&'static Embedded> {
    match game {
        Game::Ff1Pr => Some(&FF1),
        Game::Ff2Pr => Some(&FF2),
        _ => None,
    }
}

/// The embedded text of a game. `None` for a game with no data yet.
#[must_use]
pub fn sources_for(game: Game) -> Option<Sources<'static>> {
    embedded(game).map(|embedded| embedded.sources)
}

/// The parsed embedded tables of a game, parsed once and shared by every
/// caller. `None` for a game with no data yet.
///
/// # Panics
///
/// Panics when the embedded text is invalid. The text is a constant of the
/// build and the tests of this crate parse it, so a release never panics
/// here. [`GameTables::parse`] is the fallible route for other text.
#[must_use]
pub fn tables_for(game: Game) -> Option<&'static GameTables> {
    let embedded = embedded(game)?;
    Some(embedded.tables.get_or_init(|| load(game, &embedded.sources)))
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
