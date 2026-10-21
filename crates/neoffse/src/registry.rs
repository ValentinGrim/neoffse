//! The registry of shipped formats: detection, folder scan and open.

use std::fmt;
use std::path::{Path, PathBuf};

use neoffse_core::{Detection, FileProbe, FormatError, Game, PROBE_HEAD_LEN, SaveDocument, SaveFiles, SaveFormat};
use neoffse_data::Language;
use neoffse_format_pr::ff1::Ff1Format;
use neoffse_io::atomic::is_temp_name;
use neoffse_io::error::IoError;
use neoffse_io::files;

use crate::error::Error;

/// The formats NeoFFSE ships, one per supported game.
///
/// The registry is `Send + Sync`. Its `Debug` lists the games.
pub struct Registry {
    /// In `Game::ALL` order: `detect` and `open` rely on it for ties.
    formats: Vec<Box<dyn SaveFormat>>,
}

/// A format's claim on a file: its game and how sure it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Claim {
    /// The game whose format claims the file.
    pub game: Game,
    /// The format's detection: confidence and, when known, the slot.
    pub detection: Detection,
}

/// A file of a scanned folder that a format claims.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Found {
    /// The file name inside the folder. A personal value: never log it raw.
    pub name: String,
    /// The claim on the file.
    pub claim: Claim,
}

/// The result of [`Registry::scan`].
///
/// `Debug` shows file names and paths: never log it raw.
#[derive(Debug)]
#[non_exhaustive]
pub struct Scan {
    /// The claimed files, in the listing's byte order of names.
    pub saves: Vec<Found>,
    /// Entries skipped because their name is not UTF-8.
    pub non_utf8: usize,
    /// Files whose metadata or head could not be read.
    pub unreadable: Vec<IoError>,
}

/// A save the registry opened, with what the save pipeline needs later.
///
/// `Debug` shows the game and the file name only: the document and the bytes
/// are save content, and `dir` is a personal path.
#[non_exhaustive]
pub struct OpenSave {
    /// The opened document.
    pub document: Box<dyn SaveDocument>,
    /// The game of the format that opened the file.
    pub game: Game,
    /// The folder the file was read from.
    pub dir: PathBuf,
    /// The file name inside `dir`.
    pub name: String,
    /// The bytes `open` read and gave the format, for the save pipeline's
    /// check that the file did not change on disk.
    pub original: SaveFiles,
}

impl fmt::Debug for OpenSave {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenSave")
            .field("game", &self.game.id())
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry")
            .field("games", &self.games().iter().map(|game| game.id()).collect::<Vec<_>>())
            .finish()
    }
}

impl Registry {
    /// Builds the registry of the shipped formats. Today: FF I.
    ///
    /// `language` names the game data in the views the formats build.
    ///
    /// # Panics
    ///
    /// Panics as [`Ff1Format::with_language`] does: when the embedded tables
    /// are invalid, a defect of the build.
    #[must_use]
    pub fn new(language: Language) -> Self {
        Self {
            formats: vec![Box::new(Ff1Format::with_language(language))],
        }
    }

    /// The games the registry has a format for, in `Game::ALL` order.
    #[must_use]
    pub fn games(&self) -> Vec<Game> {
        Game::ALL.iter().copied().filter(|game| self.format(*game).is_some()).collect()
    }

    /// The format of `game`, or `None` when the registry has none.
    #[must_use]
    pub fn format(&self, game: Game) -> Option<&dyn SaveFormat> {
        self.formats.iter().find(|format| format.game() == game).map(|format| &**format)
    }

    /// Finds the format that claims a file.
    ///
    /// A name that [`is_temp_name`] accepts gives `None`. With
    /// `folder_game: Some(game)`, only the format of `game` answers: the six
    /// Pixel Remaster formats claim the same names, and the folder's game
    /// breaks the tie. The result is `None` when the registry has no format
    /// for `game` or that format does not claim the file. With `None`, the
    /// claim of the highest `Confidence` wins, and a tie goes to the game
    /// first in `Game::ALL`.
    #[must_use]
    pub fn detect(&self, probe: &FileProbe<'_>, folder_game: Option<Game>) -> Option<Claim> {
        if is_temp_name(probe.name) {
            return None;
        }
        let claim = |game: Game| {
            let detection = self.format(game)?.detect(probe)?;
            Some(Claim { game, detection })
        };
        if let Some(game) = folder_game {
            return claim(game);
        }
        let mut best: Option<Claim> = None;
        for claim in Game::ALL.iter().filter_map(|game| claim(*game)) {
            // Strictly greater: an equal claim of a later game never replaces it.
            if best.is_none_or(|kept| claim.detection.confidence > kept.detection.confidence) {
                best = Some(claim);
            }
        }
        best
    }

    /// Lists the files of `dir` that a format claims, through [`Registry::detect`].
    ///
    /// # Errors
    ///
    /// [`Error::Io`] when the folder cannot be listed. A file that cannot be
    /// read lands in [`Scan::unreadable`] instead.
    pub fn scan(&self, dir: &Path, folder_game: Option<Game>) -> Result<Scan, Error> {
        let listing = files::list(dir, PROBE_HEAD_LEN).map_err(|source| Error::Io { source })?;
        let saves = listing
            .files
            .iter()
            .filter_map(|file| {
                let probe = FileProbe::new(&file.name, file.len, &file.head);
                let claim = self.detect(&probe, folder_game)?;
                Some(Found {
                    name: file.name.clone(),
                    claim,
                })
            })
            .collect();
        Ok(Scan {
            saves,
            non_utf8: listing.non_utf8,
            unreadable: listing.unreadable,
        })
    }

    /// Reads the file `name` of `dir` and opens it.
    ///
    /// With `game: Some(game)`, the format of `game` opens it. With `None`,
    /// every format tries in `Game::ALL` order and the first that opens the
    /// file wins, whatever the file's name.
    ///
    /// # Errors
    ///
    /// - [`Error::Io`]: the file cannot be read, or `name` is not a plain
    ///   file name (the io layer refuses it before any disk access).
    /// - [`Error::Format`]: the format of `game` refused the file.
    /// - [`Error::UnsupportedGame`]: the registry has no format for `game`.
    /// - [`Error::NoFormat`]: no format opened the file; one attempt per
    ///   format tried.
    pub fn open(&self, dir: &Path, name: &str, game: Option<Game>) -> Result<OpenSave, Error> {
        let bytes = files::read(dir, name).map_err(|source| Error::Io { source })?;
        let mut original = SaveFiles::new();
        original.push(name, bytes).map_err(|source| Error::Format { source })?;
        let (game, document) = match game {
            Some(game) => {
                let format = self.format(game).ok_or(Error::UnsupportedGame { game })?;
                let document = format.open(&original).map_err(|source| Error::Format { source })?;
                (game, document)
            }
            None => self.open_any(&original)?,
        };
        Ok(OpenSave {
            document,
            game,
            dir: dir.to_path_buf(),
            name: name.to_owned(),
            original,
        })
    }

    fn open_any(&self, files: &SaveFiles) -> Result<(Game, Box<dyn SaveDocument>), Error> {
        let mut attempts: Vec<(Game, FormatError)> = Vec::new();
        for game in Game::ALL {
            let Some(format) = self.format(*game) else { continue };
            match format.open(files) {
                Ok(document) => return Ok((*game, document)),
                Err(error) => attempts.push((*game, error)),
            }
        }
        Err(Error::NoFormat { attempts })
    }
}
