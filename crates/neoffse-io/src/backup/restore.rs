//! Putting a backup back into a save folder.

use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

use super::list::read_manifest;
use super::manifest::{MANIFEST_FILE, Manifest};
use super::{Backup, Moment, Request, create_protecting};
use crate::atomic::{self, FileOps, RealFileOps};
use crate::dirs::DataDir;
use crate::error::{IoError, Operation};

/// What a finished restore did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Restored {
    /// The backup of the files the restore replaced, `reason` `before-restore`.
    /// `None` when no file of the backup existed in the target folder.
    pub safety: Option<Backup>,
    /// The files that did not exist in the target folder before the restore, in
    /// manifest order. Restoring `safety` does not remove them.
    pub created: Vec<String>,
}

/// A file of the backup, read into memory.
struct Staged {
    name: String,
    bytes: Vec<u8>,
}

/// Restores `backup` into `target_dir` at the time of the system clock.
///
/// # Errors
///
/// Same as [`restore_with`].
pub fn restore(data: &DataDir, game: &str, backup: &Backup, target_dir: &Path) -> Result<Restored, IoError> {
    restore_with(&RealFileOps, data, game, backup, target_dir, Moment::now())
}

/// Writes the files of `backup` over the files of `target_dir`, after a
/// backup of the files it replaces.
///
/// The call has three phases. First it checks everything and reads every
/// backup file into memory. Then it backs up the files of the backup that
/// already exist in `target_dir` (`reason` `before-restore`, the `saveType` of
/// the restored manifest). Last it writes the files in manifest order, each
/// through [`atomic::write_with`], and compares the temporary file with the
/// bytes of the backup before the rename. The folder `target_dir` must exist
/// and the restore never creates it. The restore accepts any such folder and
/// does not compare it with the `source_dir` of the manifest, which can be
/// stale; the caller derives `target_dir` from a fresh discovery or from that
/// manifest. `ops` serves the writes of the target files, the copies of the
/// safety backup and its manifest; the folder operations use `std::fs`. `now`
/// names the safety backup.
///
/// The backup being restored is not removed by the retention pass of the
/// safety backup, so the game may hold 21 readable backups until the next
/// [`create()`](super::create).
///
/// # Errors
///
/// Before the first write, the restore returns the plain error and changes
/// nothing in `target_dir`:
///
/// - [`IoError::InvalidGameId`] for a bad game id.
/// - [`IoError::Io`] with [`Operation::Read`] when `backup.json` or a backup
///   file is missing or unreadable (`ErrorKind::NotFound` when missing), or
///   when a backup file is not a regular file (`ErrorKind::InvalidInput`).
/// - [`IoError::ManifestInvalid`] when the manifest on disk breaks a rule or
///   belongs to another game.
/// - [`IoError::Io`] with [`Operation::ListDir`] when `target_dir` is not a
///   folder.
/// - [`IoError::Io`] with [`Operation::Read`] when a file name of the manifest
///   cannot be looked up in `target_dir` for a reason other than absence.
/// - The errors of [`create_with`](super::create_with) when the safety backup
///   fails; the call removes its folder.
///
/// A failure while writing is [`IoError::RestoreFailed`]: the files already
/// written keep the backup's version, the safety backup is on disk, and
/// `source` holds the `Io` or `VerificationFailed` error of the failed file.
pub fn restore_with<O: FileOps + ?Sized>(ops: &O, data: &DataDir, game: &str, backup: &Backup, target_dir: &Path, now: Moment) -> Result<Restored, IoError> {
    data.game_backups_dir(game)?;
    // A caller can edit the public fields of a `Backup`; the file on disk is
    // what the backup holds.
    let manifest = read_manifest(&backup.dir.join(MANIFEST_FILE), game)?;
    check_folder(target_dir)?;
    // Every file is read now, not inside the write loop: a missing or
    // unreadable file must refuse the restore before any write, and the
    // verification compares the temporary file with these same bytes.
    let staged = stage_files(&backup.dir, &manifest)?;
    let (present, created) = split_present(target_dir, &manifest.files)?;

    let safety = if present.is_empty() {
        None
    } else {
        let request = Request {
            game,
            source_dir: target_dir,
            files: &present,
            save_type: &manifest.save_type,
            reason: "before-restore",
        };
        Some(create_protecting(ops, data, &request, now, Some(&backup.dir))?)
    };

    match write_files(ops, target_dir, &staged) {
        Ok(()) => Ok(Restored { safety, created }),
        Err((failed_at, source)) => Err(failure(&staged, failed_at, safety, source)),
    }
}

/// `target_dir` must exist and be a folder.
fn check_folder(target_dir: &Path) -> Result<(), IoError> {
    let list_error = |source| IoError::Io {
        operation: Operation::ListDir,
        path: target_dir.to_path_buf(),
        source,
    };
    let metadata = fs::metadata(target_dir).map_err(list_error)?;
    if metadata.is_dir() {
        Ok(())
    } else {
        Err(list_error(io::Error::from(io::ErrorKind::InvalidInput)))
    }
}

/// Reads the files of the manifest from the backup folder, in manifest order.
fn stage_files(backup_dir: &Path, manifest: &Manifest) -> Result<Vec<Staged>, IoError> {
    manifest
        .files
        .iter()
        .map(|name| {
            Ok(Staged {
                name: name.clone(),
                bytes: read_regular_file(&backup_dir.join(name))?,
            })
        })
        .collect()
}

/// Reads a file that is neither a folder nor a symbolic link: a link in a
/// backup folder could point at any file the user can read.
fn read_regular_file(path: &Path) -> Result<Vec<u8>, IoError> {
    let read_error = |source| IoError::Io {
        operation: Operation::Read,
        path: path.to_path_buf(),
        source,
    };
    let metadata = fs::symlink_metadata(path).map_err(read_error)?;
    if !metadata.is_file() {
        return Err(read_error(io::Error::from(io::ErrorKind::InvalidInput)));
    }
    fs::read(path).map_err(read_error)
}

/// Splits the manifest names into those present in `target_dir` and those
/// absent. Any entry counts as present, a link included, because the lookup
/// does not follow links. The safety backup then decides: it reads a live
/// link as the file behind it, and the atomic write replaces the link with a
/// regular file. A dangling link and a folder fail the safety backup.
fn split_present<'m>(target_dir: &Path, names: &'m [String]) -> Result<(Vec<&'m str>, Vec<String>), IoError> {
    let mut present = Vec::new();
    let mut absent = Vec::new();
    for name in names {
        let path = target_dir.join(name);
        match fs::symlink_metadata(&path) {
            Ok(_) => present.push(name.as_str()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => absent.push(name.clone()),
            Err(source) => {
                return Err(IoError::Io {
                    operation: Operation::Read,
                    path,
                    source,
                });
            }
        }
    }
    Ok((present, absent))
}

/// Writes the staged files in order. On the first error it returns the index
/// of the failed file and the error.
///
/// The loop stops there and does not roll back: a rollback would write to the
/// disk that refused the last write. The safety backup is the recovery path.
fn write_files<O: FileOps + ?Sized>(ops: &O, target_dir: &Path, staged: &[Staged]) -> Result<(), (usize, IoError)> {
    for (index, file) in staged.iter().enumerate() {
        let verify = |temp: &Path| -> Result<(), Box<dyn Error + Send + Sync>> {
            if fs::read(temp)? == file.bytes {
                Ok(())
            } else {
                Err("the file written differs from the backup".into())
            }
        };
        atomic::write_with(ops, &target_dir.join(&file.name), &file.bytes, verify).map_err(|error| (index, error))?;
    }
    Ok(())
}

/// The error of a restore that stopped at `failed_at`.
fn failure(staged: &[Staged], failed_at: usize, safety: Option<Backup>, source: IoError) -> IoError {
    let names = staged.iter().map(|file| file.name.clone());
    IoError::RestoreFailed {
        written: names.clone().take(failed_at).collect(),
        remaining: names.skip(failed_at).collect(),
        safety: safety.map(Box::new),
        source: Box::new(source),
    }
}
