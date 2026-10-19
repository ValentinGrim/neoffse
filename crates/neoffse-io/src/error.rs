//! The error type of the crate: [`IoError`] and the [`Operation`] it names.

use std::fmt;
use std::path::PathBuf;

use crate::backup::Backup;

/// The step of a file operation that failed. It names the step in
/// [`IoError::Io`].
///
/// A later version may add variants, so a `match` needs a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Operation {
    /// Creating a folder.
    CreateDir,
    /// Creating a temporary file.
    CreateTemp,
    /// Reading a file.
    Read,
    /// Writing a file.
    Write,
    /// Flushing a written file to disk.
    Sync,
    /// Copying a file.
    Copy,
    /// Renaming a file.
    Rename,
    /// Removing a file or a folder.
    Remove,
    /// Listing the entries of a folder.
    ListDir,
}

impl fmt::Display for Operation {
    /// Writes a lowercase verb phrase that follows "cannot ".
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CreateDir => "create a folder",
            Self::CreateTemp => "create a temporary file",
            Self::Read => "read",
            Self::Write => "write",
            Self::Sync => "flush to disk",
            Self::Copy => "copy",
            Self::Rename => "rename",
            Self::Remove => "remove",
            Self::ListDir => "list the folder",
        })
    }
}

/// A failed file operation, check or path rule of `neoffse-io`.
///
/// The type derives `Debug` only: `std::io::Error` and the boxed errors are
/// not `PartialEq`. Tests match with `matches!`. `Display` names the path and
/// never repeats the cause, which stays in `source()`.
///
/// A later version may add variants, so a `match` needs a wildcard arm.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IoError {
    /// The operating system refused one step of a file operation.
    #[error("cannot {operation}: {path}")]
    Io {
        /// The step that failed.
        operation: Operation,
        /// The file or folder the step acted on.
        path: PathBuf,
        /// The error the operating system returned.
        source: std::io::Error,
    },
    /// The system reports no home directory, so the data folder has no
    /// location.
    #[error("the system reports no home directory, so the data folder is unknown")]
    NoDataDir,
    /// A game id cannot name a folder.
    #[error("invalid game id {game:?}: use lowercase letters, digits and hyphens")]
    InvalidGameId {
        /// The rejected id.
        game: String,
    },
    /// The check on a file written to disk failed.
    #[error("verification of {path} failed")]
    VerificationFailed {
        /// The file that failed the check.
        path: PathBuf,
        /// The error of the check, boxed so that no format crate type
        /// appears in this crate's public API.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// The backup manifest does not serialize or parse.
    #[error("invalid backup manifest: {path}")]
    ManifestInvalid {
        /// The manifest file.
        path: PathBuf,
        /// The serialization or parse error, boxed so that no third-party
        /// type appears in this crate's public API.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A restore failed while it wrote files.
    ///
    /// The files in `written` hold the backup's version, the others hold their
    /// old one. The restore does not roll back: a rollback would write to a
    /// disk that just failed. Restoring `safety` puts the old versions back.
    #[error("restore stopped after {} of {} files", .written.len(), .written.len() + .remaining.len())]
    RestoreFailed {
        /// Files already replaced by the backup's version, in write order.
        written: Vec<String>,
        /// Files not restored: the one that failed first, then the untouched
        /// ones.
        remaining: Vec<String>,
        /// The backup of the replaced files; restoring it undoes the restore.
        /// `None` when no file existed to back up.
        safety: Option<Box<Backup>>,
        /// The error of the file that failed: `Io` or `VerificationFailed`.
        source: Box<IoError>,
    },
}
