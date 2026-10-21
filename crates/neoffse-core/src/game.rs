//! The games whose saves NeoFFSE reads.
//!
//! The six variants are the Pixel Remaster titles of
//! `docs/formats/pixel-remaster.md#games`. `neoffse-format-pr`'s
//! `games::GAMES` carries the same ids.

/// A game in the edition whose saves NeoFFSE reads.
///
/// Each variant is one title in one edition (Pixel Remaster, Steam PC; see
/// `docs/formats/pixel-remaster.md#games`). A later edition adds variants, so
/// a `match` needs a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Game {
    /// Final Fantasy I, Pixel Remaster.
    Ff1Pr,
    /// Final Fantasy II, Pixel Remaster.
    Ff2Pr,
    /// Final Fantasy III, Pixel Remaster.
    Ff3Pr,
    /// Final Fantasy IV, Pixel Remaster.
    Ff4Pr,
    /// Final Fantasy V, Pixel Remaster.
    Ff5Pr,
    /// Final Fantasy VI, Pixel Remaster.
    Ff6Pr,
}

impl Game {
    /// Every game, FF I first.
    pub const ALL: &[Game] = &[Game::Ff1Pr, Game::Ff2Pr, Game::Ff3Pr, Game::Ff4Pr, Game::Ff5Pr, Game::Ff6Pr];

    /// The stable id: `ff1-pr` to `ff6-pr`.
    ///
    /// The id names the backup folder of the game, so it holds only ASCII
    /// lowercase letters, digits and `-` (the rule of `neoffse-io`'s
    /// `DataDir::game_backups_dir`, `docs/architecture.md#save-safety`).
    /// Changing an id renames the backup folders of existing users.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Game::Ff1Pr => "ff1-pr",
            Game::Ff2Pr => "ff2-pr",
            Game::Ff3Pr => "ff3-pr",
            Game::Ff4Pr => "ff4-pr",
            Game::Ff5Pr => "ff5-pr",
            Game::Ff6Pr => "ff6-pr",
        }
    }

    /// The game whose [`id`](Game::id) is `id`, compared byte for byte.
    ///
    /// The comparison is case sensitive and trims nothing.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Game> {
        Game::ALL.iter().copied().find(|game| game.id() == id)
    }

    /// The number of the Final Fantasy title: 1 to 6.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Game::Ff1Pr => 1,
            Game::Ff2Pr => 2,
            Game::Ff3Pr => 3,
            Game::Ff4Pr => 4,
            Game::Ff5Pr => 5,
            Game::Ff6Pr => 6,
        }
    }
}
