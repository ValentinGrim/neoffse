//! The data folder: where NeoFFSE keeps its own files.

use std::path::{Path, PathBuf};

use directories::ProjectDirs;

use crate::error::IoError;

const APPLICATION: &str = "neoffse";
const BACKUPS: &str = "backups";

/// Where NeoFFSE keeps its own files: the backups.
///
/// The type builds paths and creates nothing on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    /// Asks the operating system for the data folder of NeoFFSE: the XDG data
    /// home on Linux, the roaming application data on Windows.
    ///
    /// The call reads the environment and touches no file.
    ///
    /// # Errors
    ///
    /// [`IoError::NoDataDir`] when the system reports no home directory.
    pub fn platform() -> Result<Self, IoError> {
        ProjectDirs::from("", "", APPLICATION)
            .map(|dirs| Self::at(dirs.data_dir()))
            .ok_or(IoError::NoDataDir)
    }

    /// Uses `root` as the data folder, verbatim: no canonicalization, no
    /// existence check, no folder creation.
    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The data folder.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The folder that holds every backup, `<data dir>/backups`
    /// (`docs/architecture.md#save-safety`).
    #[must_use]
    pub fn backups_dir(&self) -> PathBuf {
        self.root.join(BACKUPS)
    }

    /// The backup folder of one game, `<data dir>/backups/<game>`.
    ///
    /// A valid id is non-empty and holds only ASCII lowercase letters, ASCII
    /// digits and `-`. The rule keeps the id inside the backups folder and
    /// keeps two ids from colliding on a case-insensitive file system.
    ///
    /// # Errors
    ///
    /// [`IoError::InvalidGameId`] when `game` breaks the rule.
    pub fn game_backups_dir(&self, game: &str) -> Result<PathBuf, IoError> {
        let valid = !game.is_empty() && game.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
        if valid {
            Ok(self.backups_dir().join(game))
        } else {
            Err(IoError::InvalidGameId { game: game.to_owned() })
        }
    }
}
