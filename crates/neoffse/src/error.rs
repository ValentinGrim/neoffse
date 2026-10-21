//! What the registry reports.

use neoffse_core::{FormatError, Game};
use neoffse_io::error::IoError;

/// A registry operation that failed.
///
/// `Display` never repeats the source. `Debug` and the sources can hold
/// personal paths (`IoError`) and file names: never log them raw.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A folder or a file could not be listed or read.
    #[error("cannot access the save files")]
    Io {
        /// The io error, with the path it concerns.
        source: IoError,
    },
    /// The format refused the files, or the name is not a plain file name.
    #[error("the save files do not open")]
    Format {
        /// The format's error.
        source: FormatError,
    },
    /// The caller named a game the registry has no format for.
    #[error("no format supports {}", game.id())]
    UnsupportedGame {
        /// The requested game.
        game: Game,
    },
    /// No format opens the file. Opening without a game tries every format.
    #[error("no format opens the file")]
    NoFormat {
        /// One entry per format tried, in `Game::ALL` order, with its error.
        attempts: Vec<(Game, FormatError)>,
    },
    /// The game's process is running and the caller asked to refuse.
    #[error("the game is running")]
    GameRunning {
        /// The executable names found running.
        executables: Vec<&'static str>,
    },
    /// A file differs from the bytes the session read: another program wrote
    /// it since the open.
    #[error("the save file changed on disk")]
    ChangedOnDisk {
        /// The file name, a personal value: it appears in `Debug` only.
        name: String,
    },
}
