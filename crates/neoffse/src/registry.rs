//! The registry of shipped formats: detection, folder scan and open.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use neoffse_core::{Detection, FileProbe, FormatError, Game, PROBE_HEAD_LEN, SaveDocument, SaveFiles, SaveFormat};
use neoffse_data::Language;
use neoffse_format_pr::ff1::Ff1Format;
use neoffse_io::atomic::is_temp_name;
use neoffse_io::error::IoError;
use neoffse_io::files;

use crate::error::Error;
use crate::session::Session;

/// The game, format and document of a successful open.
type Opened = (Game, Arc<dyn SaveFormat>, Box<dyn SaveDocument>);

/// The formats NeoFFSE ships, one per supported game, or the set that
/// [`Registry::from_formats`] receives.
///
/// The registry is `Send + Sync`. Its `Debug` lists the games.
pub struct Registry {
    /// One format per game, in the order given. `format` looks a game up by
    /// `SaveFormat::game`, and `detect` and `open` walk `Game::ALL`, so this
    /// order decides nothing.
    formats: Vec<Arc<dyn SaveFormat>>,
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
///
/// `Debug` prints the file name, a personal value: never log it raw.
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

impl fmt::Debug for Registry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Registry")
            .field("games", &self.games().iter().map(|game| game.id()).collect::<Vec<_>>())
            .finish()
    }
}

impl Registry {
    /// Builds the registry of the shipped formats. Each shipped game has one
    /// line here. Today: FF I.
    ///
    /// `language` names the game data in the views the formats build.
    ///
    /// # Panics
    ///
    /// Panics as [`Ff1Format::with_language`] does: when the embedded tables
    /// are invalid, a defect of the build.
    #[must_use]
    pub fn new(language: Language) -> Self {
        Self::from_formats(vec![Arc::new(Ff1Format::with_language(language))])
    }

    /// A registry that holds `formats` and no other.
    ///
    /// The registry keeps one format per game. The order given decides
    /// nothing: [`Registry::games`], [`Registry::detect`] and
    /// [`Registry::open`] follow `Game::ALL`.
    ///
    /// # Panics
    ///
    /// When two formats report the same [`Game`], with the message `two
    /// formats for <game id>`. The duplicate is a wiring defect of the
    /// caller, not data read from a file.
    #[must_use]
    pub fn from_formats(formats: Vec<Arc<dyn SaveFormat>>) -> Self {
        let mut seen: Vec<Game> = Vec::with_capacity(formats.len());
        for format in &formats {
            let game = format.game();
            assert!(!seen.contains(&game), "two formats for {}", game.id());
            seen.push(game);
        }
        Self { formats }
    }

    /// The games the registry has a format for, in `Game::ALL` order.
    #[must_use]
    pub fn games(&self) -> Vec<Game> {
        Game::ALL.iter().copied().filter(|game| self.format(*game).is_some()).collect()
    }

    /// The format of `game`, or `None` when the registry has none.
    #[must_use]
    pub fn format(&self, game: Game) -> Option<&dyn SaveFormat> {
        self.shared(game).map(|format| &**format)
    }

    fn shared(&self, game: Game) -> Option<&Arc<dyn SaveFormat>> {
        self.formats.iter().find(|format| format.game() == game)
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
        // A format that claims by content must never claim a leftover of the atomic
        // write (`docs/architecture.md#format-traits`).
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

    /// Reads the file `name` of `dir`, opens it and starts its [`Session`].
    ///
    /// With `game: Some(game)`, the format of `game` opens it. With `None`,
    /// every format tries in `Game::ALL` order and the first that opens the
    /// file wins, whatever the file's name. The registry does not rank the
    /// attempts by [`SaveFormat::detect`]: a format must refuse a file of
    /// another game, as [`SaveFormat::open`] requires, and the registry trusts
    /// that. The session starts clean, with the bytes read as its
    /// [`original`](Session::original).
    ///
    /// # Errors
    ///
    /// - [`Error::Io`]: the file cannot be read, or `name` is not a plain
    ///   file name (the io layer refuses it before any disk access).
    /// - [`Error::Format`]: the format of `game` refused the file.
    /// - [`Error::UnsupportedGame`]: the registry has no format for `game`.
    /// - [`Error::NoFormat`]: no format opened the file; one attempt per
    ///   format tried.
    pub fn open(&self, dir: &Path, name: &str, game: Option<Game>) -> Result<Session, Error> {
        let bytes = files::read(dir, name).map_err(|source| Error::Io { source })?;
        let mut original = SaveFiles::new();
        original.push(name, bytes).map_err(|source| Error::Format { source })?;
        let (game, format, document) = match game {
            Some(game) => {
                let format = self.shared(game).ok_or(Error::UnsupportedGame { game })?;
                let document = format.open(&original).map_err(|source| Error::Format { source })?;
                (game, Arc::clone(format), document)
            }
            None => self.open_any(&original)?,
        };
        Ok(Session::opened(format, document, game, dir.to_path_buf(), name.to_owned(), original))
    }

    fn open_any(&self, files: &SaveFiles) -> Result<Opened, Error> {
        let mut attempts: Vec<(Game, FormatError)> = Vec::new();
        for game in Game::ALL {
            let Some(format) = self.shared(*game) else { continue };
            match format.open(files) {
                Ok(document) => return Ok((*game, Arc::clone(format), document)),
                Err(error) => attempts.push((*game, error)),
            }
        }
        Err(Error::NoFormat { attempts })
    }
}
