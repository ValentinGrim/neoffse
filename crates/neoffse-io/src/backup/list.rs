//! Reading the backups of a game back, and keeping the newest twenty.

use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

use super::Backup;
use super::manifest::{MANIFEST_FILE, Manifest};
use crate::dirs::DataDir;
use crate::error::{IoError, Operation};

/// Backups kept per game: 20 (`docs/architecture.md#save-safety`).
pub const MAX_BACKUPS: usize = 20;

/// The backups of one game and the folders that could not be read.
#[derive(Debug)]
#[non_exhaustive]
pub struct Listing {
    /// The readable backups, newest first.
    pub backups: Vec<Backup>,
    /// One error per backup-named folder that is not a readable backup of
    /// the game, in the same order as `backups`: `Io` with
    /// [`Operation::Read`] (and `ErrorKind::NotFound` for a folder without
    /// `backup.json`) or [`IoError::ManifestInvalid`].
    pub unreadable: Vec<IoError>,
}

/// Lists the backups of `game`, newest first.
///
/// Only a directory named `YYYYMMDD-HHMMSS` or `YYYYMMDD-HHMMSS-NN` takes
/// part. A file, a symbolic link and any other name are ignored. The names
/// have a fixed width, so the descending name order is the newest-first
/// order ([`Moment::now`](super::Moment::now) writes UTC). `list` does not
/// check that the files named by a manifest exist in the folder.
///
/// A game without a backup folder lists as empty, and the call creates
/// nothing.
///
/// # Errors
///
/// - [`IoError::InvalidGameId`] when the game id breaks the rule of
///   [`DataDir::game_backups_dir`].
/// - [`IoError::Io`] with [`Operation::ListDir`] when the game folder
///   exists but cannot be listed, for example when it is a file.
///
/// A folder that is not a readable backup is not an error of the call: it
/// lands in [`Listing::unreadable`].
pub fn list(data: &DataDir, game: &str) -> Result<Listing, IoError> {
    scan(&data.game_backups_dir(game)?, game)
}

fn scan(game_dir: &Path, game: &str) -> Result<Listing, IoError> {
    let list_error = |source| IoError::Io {
        operation: Operation::ListDir,
        path: game_dir.to_path_buf(),
        source,
    };
    let mut listing = Listing {
        backups: Vec::new(),
        unreadable: Vec::new(),
    };
    let entries = match fs::read_dir(game_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(listing),
        Err(error) => return Err(list_error(error)),
    };
    let mut stamps = Vec::new();
    for entry in entries {
        let entry = entry.map_err(list_error)?;
        // `DirEntry::file_type` does not follow a symbolic link.
        if !entry.file_type().map_err(list_error)?.is_dir() {
            continue;
        }
        if let Ok(name) = entry.file_name().into_string()
            && is_stamp(&name)
        {
            stamps.push(name);
        }
    }
    stamps.sort_unstable_by(|a, b| b.cmp(a));

    for stamp in stamps {
        let dir = game_dir.join(stamp);
        match read_manifest(&dir.join(MANIFEST_FILE), game) {
            Ok(manifest) => listing.backups.push(Backup { dir, manifest }),
            Err(error) => listing.unreadable.push(error),
        }
    }
    Ok(listing)
}

/// Reads and checks one `backup.json`, which must belong to `game`.
pub(super) fn read_manifest(path: &Path, game: &str) -> Result<Manifest, IoError> {
    let invalid = |source: Box<dyn Error + Send + Sync>| IoError::ManifestInvalid {
        path: path.to_path_buf(),
        source,
    };
    let bytes = fs::read(path).map_err(|source| IoError::Io {
        operation: Operation::Read,
        path: path.to_path_buf(),
        source,
    })?;
    let manifest = Manifest::parse(&bytes).map_err(invalid)?;
    if manifest.game == game {
        Ok(manifest)
    } else {
        Err(invalid(format!("manifest belongs to game {:?}, not {game:?}", manifest.game).into()))
    }
}

/// Whether `name` is `DDDDDDDD-DDDDDD` or `DDDDDDDD-DDDDDD-DD`, in ASCII
/// digits.
fn is_stamp(name: &str) -> bool {
    let widths: Vec<usize> = name.split('-').map(str::len).collect();
    (widths == [8, 6] || widths == [8, 6, 2]) && name.bytes().all(|byte| byte == b'-' || byte.is_ascii_digit())
}

/// Deletes the oldest readable backups of the game until [`MAX_BACKUPS`]
/// remain, never `keep`. It stops at the first deletion that fails.
///
/// Only readable backups count and only they go. A folder without a
/// manifest may be a backup another process is still writing, and a manifest
/// this version cannot read may come from a newer one: a tool does not
/// delete what it cannot read.
///
/// `keep` is the backup just made. When the clock ran backwards it sorts as
/// the oldest, and the oldest of the others go in its place.
///
/// `protect` is a backup folder that stays and does not count toward
/// [`MAX_BACKUPS`]. A restore protects the backup it reads from, which would
/// otherwise be the oldest one and the first to go. The match is on the
/// folder name, because every backup of the game sits in `game_dir` and a
/// caller may spell the data root differently when it lists and when it
/// restores (`..`, a symbolic link). A false match keeps one extra backup.
pub(super) fn prune(game_dir: &Path, keep: &Backup, protect: Option<&Path>) -> Result<(), IoError> {
    let valid = scan(game_dir, &keep.manifest.game)?.backups;
    let name = protect.and_then(Path::file_name);
    let is_protected = |backup: &Backup| name.is_some() && backup.dir.file_name() == name;
    let protected = usize::from(valid.iter().any(is_protected));
    let excess = valid.len().saturating_sub(protected).saturating_sub(MAX_BACKUPS);
    let candidates = valid.iter().rev().filter(|backup| backup.dir != keep.dir && !is_protected(backup));
    for old in candidates.take(excess) {
        fs::remove_dir_all(&old.dir).map_err(|source| IoError::Io {
            operation: Operation::Remove,
            path: old.dir.clone(),
            source,
        })?;
    }
    Ok(())
}
