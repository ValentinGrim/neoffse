//! Verified replacement of a file.
//!
//! [`write()`] puts `bytes` in a temporary file inside the target's own folder,
//! flushes it to disk, lets the caller check that file, then renames it over
//! the target. The target keeps its previous content or holds all of
//! `bytes`, with no state in between. Any failure before the rename removes
//! the temporary file.
//!
//! Each file operation goes through [`FileOps`], so a test can make any one
//! step fail. [`RealFileOps`] is the operating system implementation.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use crate::error::{IoError, Operation};

/// The fixed prefix and suffix of a temporary name mark a stray file, so a
/// listing can filter on them; the random part sits between. A PR save file
/// has no extension (`docs/formats/pixel-remaster.md#slot-files`).
const TEMP_PREFIX: &str = ".neoffse-";
const TEMP_SUFFIX: &str = ".tmp";

/// The file operations of the write pipeline, one method per step.
pub trait FileOps {
    /// Creates an empty file with a unique name inside `dir` and returns it
    /// open for writing, with its path. The file stays on disk until
    /// `remove_file` or a rename moves it.
    ///
    /// # Errors
    ///
    /// Returns the operating system error, for example when `dir` does not
    /// exist.
    fn create_temp(&self, dir: &Path) -> io::Result<(File, PathBuf)>;

    /// Writes all of `bytes` to `file`.
    ///
    /// # Errors
    ///
    /// Returns the operating system error.
    fn write_all(&self, file: &mut File, bytes: &[u8]) -> io::Result<()>;

    /// Gives `file` the permissions of the file at `from`. When nothing
    /// exists at `from` (`ErrorKind::NotFound`), or when `from` is not a
    /// file (a folder), changes nothing and returns `Ok`. A folder at the
    /// target then fails the write at the rename on every system, because
    /// Windows refuses to give a file the attributes of a folder.
    ///
    /// # Errors
    ///
    /// Returns any operating system error other than `NotFound`.
    fn copy_permissions(&self, from: &Path, file: &File) -> io::Result<()>;

    /// Flushes the content and the metadata of `file` to the disk.
    ///
    /// # Errors
    ///
    /// Returns the operating system error.
    fn sync_file(&self, file: &File) -> io::Result<()>;

    /// Renames `from` to `to`, replacing an existing file at `to`.
    ///
    /// # Errors
    ///
    /// Returns the operating system error, for example when `to` is a
    /// folder or sits on another file system.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Removes the file at `path`.
    ///
    /// # Errors
    ///
    /// Returns the operating system error.
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Flushes the folder entry changes of `dir` to the disk.
    ///
    /// # Errors
    ///
    /// Returns the operating system error.
    fn sync_dir(&self, dir: &Path) -> io::Result<()>;

    /// Copies the content of the file at `from` into a new file at `to`, then
    /// flushes the new file through [`sync_file`](Self::sync_file).
    ///
    /// The default implementation opens `from`, creates `to` with
    /// `create_new` (an existing file is never overwritten), copies the bytes
    /// with [`io::copy`] and calls `self.sync_file` on the new file. A test
    /// implementor that fails `sync_file` therefore fails the copy.
    ///
    /// # Errors
    ///
    /// Returns the operating system error of the step that failed, for
    /// example `NotFound` for a missing `from` and `AlreadyExists` when `to`
    /// exists.
    fn copy_file(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut source = File::open(from)?;
        let mut destination = OpenOptions::new().write(true).create_new(true).open(to)?;
        io::copy(&mut source, &mut destination)?;
        self.sync_file(&destination)
    }
}

/// The operating system implementation of [`FileOps`].
#[derive(Debug, Clone, Copy, Default)]
pub struct RealFileOps;

impl FileOps for RealFileOps {
    /// The name looks like `.neoffse-AbC123.tmp`. On Unix the file starts
    /// with mode `0o600` before the umask.
    fn create_temp(&self, dir: &Path) -> io::Result<(File, PathBuf)> {
        let named = tempfile::Builder::new().prefix(TEMP_PREFIX).suffix(TEMP_SUFFIX).tempfile_in(dir)?;
        named.keep().map_err(|persist| persist.error)
    }

    fn write_all(&self, file: &mut File, bytes: &[u8]) -> io::Result<()> {
        file.write_all(bytes)
    }

    /// `metadata` follows a symbolic link, so a link at `from` gives the
    /// mode of the file behind it. On Unix the call is `fchmod`, which the
    /// umask does not alter. On Windows only the read-only flag moves.
    /// Only a regular file passes its permissions on.
    fn copy_permissions(&self, from: &Path, file: &File) -> io::Result<()> {
        match fs::metadata(from) {
            Ok(metadata) if metadata.is_file() => file.set_permissions(metadata.permissions()),
            Ok(_) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn sync_file(&self, file: &File) -> io::Result<()> {
        file.sync_all()
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    /// Flushes on Unix. `std` cannot open a folder for flushing on Windows,
    /// so the call does nothing there.
    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        #[cfg(unix)]
        {
            File::open(dir)?.sync_all()
        }
        #[cfg(not(unix))]
        {
            let _ = dir;
            Ok(())
        }
    }
}

/// Removes the temporary file when dropped, unless disarmed. It runs on
/// every `Err` return and on a panic in the caller's hook.
struct TempGuard<'a, O: FileOps + ?Sized> {
    ops: &'a O,
    path: PathBuf,
    armed: bool,
}

impl<O: FileOps + ?Sized> Drop for TempGuard<'_, O> {
    fn drop(&mut self) {
        if self.armed {
            // The caller needs the error of the failed step, not this one.
            let _ = self.ops.remove_file(&self.path);
        }
    }
}

/// Replaces the file at `target` with `bytes` through [`RealFileOps`].
///
/// The target either keeps its previous content or holds `bytes`. The
/// function never creates a missing folder. An existing target passes its
/// permissions to the new file; a new target gets the default of the
/// `tempfile` crate (`0o600` on Unix). A symbolic link at the target is
/// replaced, not followed.
///
/// `verify` receives the path of the written temporary file, which lies in
/// the target's folder. It must read that file itself. The target still
/// holds its previous content during the call, and a hook error leaves it so.
///
/// # Errors
///
/// Returns [`IoError::Io`] when a file operation fails (the `operation`
/// field names the step, and `CreateTemp` names the folder), and
/// [`IoError::VerificationFailed`] when `verify` fails. The call removes the
/// temporary file, and a failed removal never replaces the error of the step.
pub fn write<V, E>(target: &Path, bytes: &[u8], verify: V) -> Result<(), IoError>
where
    V: FnOnce(&Path) -> Result<(), E>,
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    write_with(&RealFileOps, target, bytes, verify)
}

/// The pipeline of [`write()`] over any [`FileOps`]. It exists for failure
/// injection in tests; callers use [`write()`].
///
/// The steps run in this order: `create_temp`, `write_all`,
/// `copy_permissions`, `sync_file`, `verify`, `rename`, `sync_dir`. A failure
/// of `sync_dir` after the rename does not fail the call.
///
/// # Errors
///
/// Same as [`write()`]. `copy_permissions` failures report
/// [`Operation::Write`].
pub fn write_with<O, V, E>(ops: &O, target: &Path, bytes: &[u8], verify: V) -> Result<(), IoError>
where
    O: FileOps + ?Sized,
    V: FnOnce(&Path) -> Result<(), E>,
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let dir = folder_of(target)?;
    let (file, temp_path) = ops.create_temp(dir).map_err(|source| IoError::Io {
        operation: Operation::CreateTemp,
        path: dir.to_path_buf(),
        source,
    })?;
    let mut guard = TempGuard {
        ops,
        path: temp_path,
        armed: true,
    };

    fill(ops, target, bytes, file)?;
    verify(&guard.path).map_err(|error| IoError::VerificationFailed {
        path: target.to_path_buf(),
        source: error.into(),
    })?;
    ops.rename(&guard.path, target).map_err(|source| IoError::Io {
        operation: Operation::Rename,
        path: target.to_path_buf(),
        source,
    })?;
    guard.armed = false;

    // The rename already replaced the target; a failed folder flush cannot
    // undo it.
    let _ = ops.sync_dir(dir);
    Ok(())
}

/// The folder that holds `target`; `.` for a bare file name.
fn folder_of(target: &Path) -> Result<&Path, IoError> {
    match target.parent() {
        Some(parent) if parent.as_os_str().is_empty() => Ok(Path::new(".")),
        Some(parent) => Ok(parent),
        None => Err(IoError::Io {
            operation: Operation::CreateTemp,
            path: target.to_path_buf(),
            source: io::Error::new(io::ErrorKind::InvalidInput, "the target has no folder part"),
        }),
    }
}

/// Writes the bytes, copies the permissions, flushes and closes. `file` drops
/// when the function returns or unwinds, before the caller's guard removes
/// the path, because Windows cannot remove an open file.
fn fill<O: FileOps + ?Sized>(ops: &O, target: &Path, bytes: &[u8], mut file: File) -> Result<(), IoError> {
    let step = |operation| {
        move |source| IoError::Io {
            operation,
            path: target.to_path_buf(),
            source,
        }
    };
    ops.write_all(&mut file, bytes).map_err(step(Operation::Write))?;
    ops.copy_permissions(target, &file).map_err(step(Operation::Write))?;
    ops.sync_file(&file).map_err(step(Operation::Sync))
}
