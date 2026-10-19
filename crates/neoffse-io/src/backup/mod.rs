//! Timestamped backups of a game's save files.
//!
//! [`create()`] copies the files of a request from the save folder into
//! `<data dir>/backups/<game>/<YYYYMMDD-HHMMSS>/` and writes `backup.json`
//! ([`Manifest`]) last, through [`atomic::write_with`]. The manifest appears
//! whole or not at all, so a folder that holds `backup.json` is a complete
//! backup. A failed call removes the folder it claimed.
//!
//! Two backups in the same second get a suffix: `<stamp>-01`, then `-02`, up
//! to `-99`. A hundredth backup in one second fails with
//! `ErrorKind::AlreadyExists`. The stamp is the local time of the [`Moment`];
//! [`Moment::now`] gives UTC.
//!
//! This version keeps every backup and deletes none.
//!
//! The file `backup.json` of a backup folder lists the files by plain name.
//! The path of file `name` in a [`Backup`] is `backup.dir.join(name)`, for a
//! `name` in `backup.manifest.files`.
//!
//! `source_dir` holds a path from the user's home, and the manifest keeps it
//! on the local disk. No function of this module logs it or prints it in an
//! error.

mod manifest;

use std::error::Error;
use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use crate::atomic::{self, FileOps, RealFileOps};
use crate::dirs::DataDir;
use crate::error::{IoError, Operation};

pub use manifest::{MANIFEST_VERSION, Manifest, Moment};

use manifest::MANIFEST_FILE;

/// The highest collision suffix of a backup folder name.
const MAX_SUFFIX: u32 = 99;

/// What to back up.
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
/// [`FileOps`] for the manifest write. It exists for tests; callers use
/// [`create()`]. The copies and the folder operations use `std::fs`.
///
/// # Errors
///
/// - [`IoError::InvalidGameId`] when the game id breaks the rule of
///   [`DataDir::game_backups_dir`].
/// - [`IoError::ManifestInvalid`] when the request breaks a manifest rule
///   (no file, a name that is not a plain file name, a repeated name, an
///   empty `save_type` or `reason`). Nothing exists on disk yet.
/// - [`IoError::Io`] when a folder or a copy fails: `CreateDir` for the game
///   folder or the backup folder (all 100 names taken: `AlreadyExists`),
///   `Read` for a source file that does not open, `Write` for a destination
///   that does not open, `Copy` for the transfer, `Sync` for the flush.
/// - [`IoError::VerificationFailed`] when the manifest read back from its
///   temporary file differs from the one written, and [`IoError::Io`] for a
///   failed step of the manifest write.
///
/// After the backup folder exists, every error removes it.
pub fn create_with<O: FileOps + ?Sized>(ops: &O, data: &DataDir, request: &Request<'_>, now: Moment) -> Result<Backup, IoError> {
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

    let _ = ops.sync_dir(&game_dir);
    Ok(Backup { dir, manifest })
}

/// Creates the first free folder of `<stamp>`, `<stamp>-01` ... `<stamp>-99`.
///
/// The exclusive `create_dir` settles two callers that race for the same
/// second: the loser sees `AlreadyExists` and takes the next name. An
/// existence check followed by a create would let both win.
fn claim_folder(game_dir: &Path, stamp: &str) -> Result<PathBuf, IoError> {
    for suffix in 0..=MAX_SUFFIX {
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

/// Copies the files, then writes the manifest.
///
/// The manifest comes last and goes through the atomic pipeline: a folder
/// that holds `backup.json` holds every file it names. The verification
/// reads the temporary file back and compares it with the manifest in memory.
fn fill_folder<O: FileOps + ?Sized>(ops: &O, dir: &Path, request: &Request<'_>, manifest: &Manifest) -> Result<(), IoError> {
    for name in request.files {
        copy_file(&request.source_dir.join(name), &dir.join(name))?;
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

/// Copies the bytes of `from` into a new file `to` and flushes it to disk.
fn copy_file(from: &Path, to: &Path) -> Result<(), IoError> {
    let step = |operation, path: &Path| {
        let path = path.to_path_buf();
        move |source| IoError::Io { operation, path, source }
    };
    let mut source = File::open(from).map_err(step(Operation::Read, from))?;
    let mut destination = OpenOptions::new().write(true).create_new(true).open(to).map_err(step(Operation::Write, to))?;
    io::copy(&mut source, &mut destination).map_err(step(Operation::Copy, from))?;
    destination.sync_all().map_err(step(Operation::Sync, to))
}
