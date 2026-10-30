//! Timestamped backups of a game's save files.
//!
//! [`create()`] copies the files of a request from the save folder into
//! `<data dir>/backups/<game>/<YYYYMMDD-HHMMSS>/` and writes `backup.json`
//! ([`Manifest`]) last, through [`atomic::write_with`]. The manifest appears
//! whole or not at all, so a folder that holds `backup.json` is a complete
//! backup. A failed call removes the folder it claimed.
//!
//! Two backups in the same second get a suffix: `<stamp>-01`, then `-02`, up
//! to `-99`. A call takes the name after the highest suffix on disk, so a name
//! that retention freed is not reused in that second. The 101st backup in one
//! second fails with `ErrorKind::AlreadyExists`, even when retention left
//! fewer folders. The stamp is the UTC time of the [`Moment`] whatever its
//! offset; `created` in the manifest keeps the offset.
//!
//! [`list()`] reads the backups of a game back, newest first. A folder
//! whose name is a backup stamp but that holds no readable `backup.json` is
//! incomplete or invalid: `list` reports it in [`Listing::unreadable`] and
//! leaves it on disk.
//!
//! After a successful [`create()`], the oldest backups go until
//! [`MAX_BACKUPS`] (20) remain for the game. Only readable backups count and
//! only they are deleted. The backup just made always stays.
//!
//! [`restore()`] writes the files of one backup over the files of a save
//! folder. It reads the manifest and every backup file first, backs up the
//! current versions of the files it will replace (`reason`
//! `before-restore`), then writes them one by one through the verified atomic
//! write. A failure before the first write returns the plain error and
//! changes nothing; a failure after it is [`IoError::RestoreFailed`], which
//! carries the safety backup as the way back. The backup being restored is
//! exempt from the retention pass of the safety backup, so a game may hold 21
//! readable backups until the next [`create()`].
//!
//! The file `backup.json` of a backup folder lists the files by plain name.
//! The path of file `name` in a [`Backup`] is `backup.dir.join(name)`, for a
//! `name` in `backup.manifest.files`.
//!
//! `source_dir` holds a path from the user's home, and the manifest keeps it
//! on the local disk. An error names the failing file, whose path starts with
//! `source_dir`, with the target folder of a restore or with the data folder,
//! so a caller must not log the `Display` or the `Debug` of an error of this
//! module.

mod list;
mod manifest;
mod restore;

use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::atomic::{self, FileOps, RealFileOps};
use crate::dirs::DataDir;
use crate::error::{IoError, Operation};

pub use list::{Listing, MAX_BACKUPS, list};
pub use manifest::{MANIFEST_VERSION, Manifest, Moment};
pub use restore::{Restored, restore, restore_with};

use manifest::MANIFEST_FILE;

/// The highest collision suffix of a backup folder name: `[-NN]` has two
/// digits (`docs/architecture.md#save-safety`).
const MAX_SUFFIX: u32 = 99;

/// What to back up.
///
/// `Debug` prints `source_dir`, a path under the user's home: never log it.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// The game id, checked by [`DataDir::game_backups_dir`].
    pub game: &'a str,
    /// The folder that holds the files.
    pub source_dir: &'a Path,
    /// Plain file names inside `source_dir`.
    pub files: &'a [&'a str],
    /// The `saveType` of the manifest, non-empty, for example `slot`.
    pub save_type: &'a str,
    /// The `reason` of the manifest, non-empty, for example `before-save` or
    /// `before-restore`.
    pub reason: &'a str,
}

/// One backup on disk.
///
/// The `Debug` output holds `dir` and `manifest.source_dir`, paths under the
/// user's home: do not log it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Backup {
    /// The backup folder, `<data dir>/backups/<game>/<stamp>[-NN]`.
    pub dir: PathBuf,
    /// The content of its `backup.json`.
    pub manifest: Manifest,
}

/// Backs up the files of `request` at the time of the system clock.
///
/// # Errors
///
/// Same as [`create_with`].
pub fn create(data: &DataDir, request: &Request<'_>) -> Result<Backup, IoError> {
    create_with(&RealFileOps, data, request, Moment::now())
}

/// The steps of [`create()`] with a chosen instant and a chosen
/// [`FileOps`] for the copies and the manifest write. It exists for tests;
/// callers use [`create()`]. The folder operations use `std::fs`.
///
/// # Errors
///
/// - [`IoError::InvalidGameId`] when the game id breaks the rule of
///   [`DataDir::game_backups_dir`].
/// - [`IoError::ManifestInvalid`] when the request breaks a manifest rule
///   (no file, a name that is not a plain file name, a repeated name, an
///   empty `save_type` or `reason`). Nothing exists on disk yet.
/// - [`IoError::Io`] when a folder or a copy fails: `CreateDir` for the game
///   folder or the backup folder (no name left for the second:
///   `AlreadyExists`), `ListDir` for the scan of the game folder, `Copy` for
///   any failed copy (the source does not open, the destination exists or
///   does not open, the transfer or the flush fails), with the source file as
///   path.
/// - [`IoError::VerificationFailed`] when the manifest read back from its
///   temporary file differs from the one written, and [`IoError::Io`] for a
///   failed step of the manifest write.
///
/// After the backup folder exists, every error removes it. A success then
/// deletes the oldest readable backups beyond [`MAX_BACKUPS`]; a failure of
/// that cleanup is not reported.
pub fn create_with<O: FileOps + ?Sized>(ops: &O, data: &DataDir, request: &Request<'_>, now: Moment) -> Result<Backup, IoError> {
    create_protecting(ops, data, request, now, None)
}

/// The body of [`create_with`]. `protect` names a backup folder that the
/// retention pass leaves alone and does not count: a restore passes the
/// folder it reads from.
fn create_protecting<O: FileOps + ?Sized>(ops: &O, data: &DataDir, request: &Request<'_>, now: Moment, protect: Option<&Path>) -> Result<Backup, IoError> {
    let game_dir = data.game_backups_dir(request.game)?;
    let stamp = now.stamp();
    let manifest = Manifest {
        version: MANIFEST_VERSION,
        game: request.game.to_owned(),
        source_dir: request.source_dir.to_string_lossy().into_owned(),
        created: now.rfc3339(),
        files: request.files.iter().map(|name| (*name).to_owned()).collect(),
        save_type: request.save_type.to_owned(),
        reason: request.reason.to_owned(),
    };
    manifest.validate().map_err(|source| IoError::ManifestInvalid {
        path: game_dir.join(&stamp).join(MANIFEST_FILE),
        source,
    })?;

    fs::create_dir_all(&game_dir).map_err(|source| IoError::Io {
        operation: Operation::CreateDir,
        path: game_dir.clone(),
        source,
    })?;
    let dir = claim_folder(&game_dir, &stamp)?;
    if let Err(error) = fill_folder(ops, &dir, request, &manifest) {
        // The caller needs the error of the failed step, not this one.
        let _ = fs::remove_dir_all(&dir);
        return Err(error);
    }

    let backup = Backup { dir, manifest };
    // The backup exists and the save pipeline goes on. A failed deletion
    // leaves an extra old backup, and the next successful call prunes again.
    let _ = list::prune(&game_dir, &backup, protect);
    // fill_folder synced the files and the manifest. A failed sync here risks only the
    // new folder's entry in the game folder after a power cut, and the backup is already made.
    let _ = ops.sync_dir(&game_dir);
    Ok(backup)
}

/// Creates the folder after the highest `<stamp>[-NN]` name on disk.
///
/// A name is never taken twice in one second, whatever retention deleted in
/// between: the freed bare stamp would sort as the oldest backup and the next
/// retention pass would delete a newer one. The 101st call in one second
/// fails with `AlreadyExists`, even when fewer folders remain.
///
/// The exclusive `create_dir` settles two callers that scan the same folder:
/// the loser sees `AlreadyExists` and takes the next name. An existence check
/// followed by a create would let both win.
fn claim_folder(game_dir: &Path, stamp: &str) -> Result<PathBuf, IoError> {
    for suffix in next_suffix(game_dir, stamp)?..=MAX_SUFFIX {
        let candidate = if suffix == 0 {
            game_dir.join(stamp)
        } else {
            game_dir.join(format!("{stamp}-{suffix:02}"))
        };
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(source) => {
                return Err(IoError::Io {
                    operation: Operation::CreateDir,
                    path: candidate,
                    source,
                });
            }
        }
    }
    Err(IoError::Io {
        operation: Operation::CreateDir,
        path: game_dir.join(format!("{stamp}-{MAX_SUFFIX}")),
        source: io::Error::from(io::ErrorKind::AlreadyExists),
    })
}

/// The suffix after the highest one used by an entry of `game_dir` named
/// `<stamp>` (suffix 0) or `<stamp>-NN` (`NN` is two ASCII digits), or 0 when
/// none exists. Files count too, because `create_dir` collides with them.
fn next_suffix(game_dir: &Path, stamp: &str) -> Result<u32, IoError> {
    let list_error = |source| IoError::Io {
        operation: Operation::ListDir,
        path: game_dir.to_path_buf(),
        source,
    };
    let mut next = 0;
    for entry in fs::read_dir(game_dir).map_err(list_error)? {
        let entry = entry.map_err(list_error)?;
        let name = entry.file_name();
        let Some(rest) = name.to_str().and_then(|name| name.strip_prefix(stamp)) else {
            continue;
        };
        let suffix = match rest.strip_prefix('-') {
            None if rest.is_empty() => 0,
            Some(digits) if digits.len() == 2 && digits.bytes().all(|byte| byte.is_ascii_digit()) => digits.parse().unwrap_or(0),
            _ => continue,
        };
        next = next.max(suffix + 1);
    }
    Ok(next)
}

/// Copies the files, then writes the manifest.
///
/// The manifest comes last and goes through the atomic pipeline: a folder
/// that holds `backup.json` holds every file it names. The verification
/// reads the temporary file back and compares it with the manifest in memory.
fn fill_folder<O: FileOps + ?Sized>(ops: &O, dir: &Path, request: &Request<'_>, manifest: &Manifest) -> Result<(), IoError> {
    for name in request.files {
        let from = request.source_dir.join(name);
        ops.copy_file(&from, &dir.join(name)).map_err(|source| IoError::Io {
            operation: Operation::Copy,
            path: from,
            source,
        })?;
    }

    let manifest_path = dir.join(MANIFEST_FILE);
    let mut bytes = serde_json::to_vec_pretty(manifest).map_err(|error| IoError::ManifestInvalid {
        path: manifest_path.clone(),
        source: error.into(),
    })?;
    bytes.push(b'\n');
    atomic::write_with(ops, &manifest_path, &bytes, |temp: &Path| -> Result<(), Box<dyn Error + Send + Sync>> {
        let written = Manifest::parse(&fs::read(temp)?)?;
        if written == *manifest {
            Ok(())
        } else {
            Err("the manifest read back differs from the manifest written".into())
        }
    })
}
