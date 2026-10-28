//! Finding the save folders and the installs of Steam games.
//!
//! A format crate hands [`discover`] its [`SteamGame`] descriptors and the
//! caller fills an [`Environment`]. Discovery reads the disk and never
//! writes or creates anything. It knows no save format: it returns every
//! save folder `<Documents>/My Games/<folder>/Steam/<SteamID64>` that exists,
//! with what produced it, every [`Install`] a Steam library holds, and lists
//! the places it had to skip.
//!
//! Nothing in the crate logs or prints a path or a `SteamID64`. The `Debug`
//! output of [`Discovery`] and [`SaveFolder`] holds both, so a caller must
//! not log it either. The rules behind the search are in
//! `docs/formats/pixel-remaster.md#steam-roots-and-libraries`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, UserDirs};

use crate::error::{IoError, Operation};

pub mod library_list;
mod linux;
mod steam;
mod windows;

/// What a format crate declares about the save folder of one Steam game:
/// `<Documents>/My Games/<folder>/Steam/<SteamID64>/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SteamGame<'a> {
    /// The NeoFFSE game id, for example `ff1-pr`. Discovery copies it into the results.
    pub id: &'a str,
    /// The Steam app ID, for example 1173770.
    pub app_id: u32,
    /// The folder name under `My Games`. One plain folder name, joined verbatim.
    pub folder: &'a str,
}

/// The values discovery reads from the environment, filled by the caller.
///
/// Every path must be absolute. [`discover`] treats a relative path as
/// absent: it searches nothing there and reports nothing, because a relative
/// path would resolve against the current folder of the process.
/// `home: None` switches off the Linux search; each Windows candidate needs
/// its own field.
///
/// The `Debug` output holds personal paths: do not log it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Environment {
    /// The user's home directory (Linux searches).
    pub home: Option<PathBuf>,
    /// The XDG data home, `$XDG_DATA_HOME` or `<home>/.local/share` (Linux).
    pub data_home: Option<PathBuf>,
    /// The Documents known folder (Windows).
    pub documents: Option<PathBuf>,
    /// `%USERPROFILE%` (Windows).
    pub user_profile: Option<PathBuf>,
    /// `%OneDrive%` (Windows).
    pub one_drive: Option<PathBuf>,
    /// `%ProgramFiles(x86)%` (Windows). The Steam root is its `Steam` folder.
    pub program_files_x86: Option<PathBuf>,
}

impl Environment {
    /// Reads the real environment. Touches no file, creates nothing.
    ///
    /// Outside Windows it fills `home` and `data_home`; on Windows it fills
    /// `documents`, `user_profile`, `one_drive` and `program_files_x86`, and
    /// an empty variable counts as unset. The other fields stay `None`, so each system runs
    /// its own search only.
    #[must_use]
    pub fn platform() -> Self {
        let mut environment = Self::default();
        if cfg!(windows) {
            environment.documents = UserDirs::new().and_then(|dirs| dirs.document_dir().map(Path::to_path_buf));
            environment.user_profile = variable("USERPROFILE");
            environment.one_drive = variable("OneDrive");
            environment.program_files_x86 = variable("ProgramFiles(x86)");
        } else if let Some(dirs) = BaseDirs::new() {
            environment.home = Some(dirs.home_dir().to_path_buf());
            environment.data_home = Some(dirs.data_dir().to_path_buf());
        }
        environment
    }
}

fn variable(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).filter(|value| !value.is_empty()).map(PathBuf::from)
}

/// What produced a save folder.
///
/// A later version may add variants, so a `match` needs a wildcard arm.
///
/// `Debug` prints Steam and Documents folders, personal paths: never log it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Origin {
    /// A Proton prefix inside a Steam library (Linux).
    SteamLibrary {
        /// The canonical Steam root whose library list named the library, or that is the library.
        root: PathBuf,
        /// The canonical library that holds `steamapps/compatdata/<app ID>`.
        library: PathBuf,
    },
    /// A Documents folder (Windows), as the first candidate that named it spelled it.
    Documents {
        /// The Documents folder, not canonicalized.
        documents: PathBuf,
    },
}

/// One save folder found on disk.
///
/// `Debug` prints a personal path and a `SteamID64`: never log it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SaveFolder {
    /// `SteamGame::id`.
    pub game: String,
    /// `SteamGame::app_id`.
    pub app_id: u32,
    /// The 17-digit folder name, a `SteamID64` (personal data).
    pub steam_id: String,
    /// `.../My Games/<folder>/Steam/<steam_id>`.
    pub path: PathBuf,
    /// What produced the folder.
    pub origin: Origin,
}

/// One game that a Steam library installs.
///
/// A library installs a game when it holds `steamapps/appmanifest_<app ID>.acf`.
/// `path` is a personal path: never log it. On Windows it can carry the `\\?\`
/// prefix of `fs::canonicalize`, because library paths are canonicalized, so a
/// caller must not display it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Install {
    /// `SteamGame::id`.
    pub game: String,
    /// `SteamGame::app_id`.
    pub app_id: u32,
    /// `<library>/steamapps/common/<folder>`. The folder can be absent while a download runs.
    pub path: PathBuf,
}

/// Everything one discovery run found and skipped.
///
/// The `Debug` output holds personal paths and `SteamID64` values: do not log it.
#[derive(Debug)]
#[non_exhaustive]
pub struct Discovery {
    /// Every save folder: the Linux hits first (roots in candidate order,
    /// libraries in the order read, games in the order given, `SteamID64`
    /// ascending), then the Windows hits (candidates in order, games in
    /// order, ids ascending). Never a duplicate path: when two descriptors
    /// name one folder, the first one gives its id.
    pub folders: Vec<SaveFolder>,
    /// Every installed game: the Linux libraries first (roots in candidate
    /// order, libraries in the order read, games in the order given), then
    /// the Windows libraries. Never a duplicate `path`, because two roots can
    /// list one library.
    pub installs: Vec<Install>,
    /// What discovery could not read, in the order it met it. A path that
    /// does not exist is not a failure and is not listed here. The `Display`
    /// of each error names a personal path: do not log it.
    pub skipped: Vec<IoError>,
}

/// Runs the Linux search, then the Windows search, over `games`.
///
/// Each search runs when the fields it needs are `Some`. The call chooses
/// nothing among several `SteamID64` folders and merges nothing: the caller
/// decides. An empty `games` slice, an empty [`Environment`] and a home
/// without Steam all give an empty [`Discovery`].
#[must_use]
pub fn discover(environment: &Environment, games: &[SteamGame<'_>]) -> Discovery {
    let mut found = Discovery {
        folders: Vec::new(),
        installs: Vec::new(),
        skipped: Vec::new(),
    };
    let environment = absolute_only(environment);
    linux::search(&environment, games, &mut found);
    windows::search(&environment, games, &mut found);
    found
}

/// A copy of `environment` where every relative path is `None`.
fn absolute_only(environment: &Environment) -> Environment {
    let keep = |path: &Option<PathBuf>| path.clone().filter(|path| path.is_absolute());
    Environment {
        home: keep(&environment.home),
        data_home: keep(&environment.data_home),
        documents: keep(&environment.documents),
        user_profile: keep(&environment.user_profile),
        one_drive: keep(&environment.one_drive),
        program_files_x86: keep(&environment.program_files_x86),
    }
}

/// Whether `name` has the shape of a `SteamID64` folder: 17 ASCII digits, the first four `7656`.
///
/// `docs/formats/pixel-remaster.md#save-folder` gives the pattern
/// `^7656\d{13}$`. The check is ASCII only: a Unicode digit does not count.
#[must_use]
pub fn is_steam_id64(name: &str) -> bool {
    name.len() == 17 && name.starts_with("7656") && name.bytes().all(|byte| byte.is_ascii_digit())
}

/// The one rule for a failed lookup. An absent path (`NotFound`, or
/// `NotADirectory` when a parent is a file) is normal: an uninstalled
/// launcher, an unmounted drive, a missing game. It gives `None` and no
/// report. Any other failure is reported once in `skipped` and gives `None`
/// too, so the caller goes on with the next candidate.
pub(crate) fn settle<T>(result: io::Result<T>, operation: Operation, path: &Path, skipped: &mut Vec<IoError>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            if !matches!(error.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory) {
                skipped.push(IoError::Io {
                    operation,
                    path: path.to_path_buf(),
                    source: error,
                });
            }
            None
        }
    }
}

/// Adds every `SteamID64` folder of `steam_dir` that belongs to `game` to
/// `found.folders`, in ascending order, with `origin`.
///
/// An entry counts when its name is valid UTF-8, passes [`is_steam_id64`] and
/// is a folder once links are followed. A file, a dangling link and any other
/// name are ignored. A link whose target cannot be examined for another
/// reason (a loop, a permission) is reported in `skipped` as a failed read of
/// its path. A folder that an earlier descriptor already returned is not
/// returned again.
pub(crate) fn probe(steam_dir: &Path, game: &SteamGame<'_>, origin: &Origin, found: &mut Discovery) {
    let Some(entries) = settle(fs::read_dir(steam_dir), Operation::ListDir, steam_dir, &mut found.skipped) else {
        return;
    };
    let mut ids = Vec::new();
    for entry in entries {
        let Some(entry) = settle(entry, Operation::ListDir, steam_dir, &mut found.skipped) else {
            continue;
        };
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !is_steam_id64(&name) {
            continue;
        }
        let path = entry.path();
        let status = settle(fs::metadata(&path), Operation::Read, &path, &mut found.skipped);
        if status.is_some_and(|status| status.is_dir()) {
            ids.push(name);
        }
    }
    ids.sort();
    for steam_id in ids {
        let path = steam_dir.join(&steam_id);
        if found.folders.iter().any(|folder| folder.path == path) {
            continue;
        }
        found.folders.push(SaveFolder {
            game: game.id.to_owned(),
            app_id: game.app_id,
            path,
            steam_id,
            origin: origin.clone(),
        });
    }
}
