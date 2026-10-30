//! Listing and reading the files of a save folder.
//!
//! The facade lists a save folder, reads the files it finds and hands their
//! bytes to a format crate. Every disk access stays in this crate, and a
//! format crate never touches the disk (`docs/architecture.md#layers-and-crates`).
//!
//! [`list()`] reports each regular file with its size and its first bytes,
//! enough for a format to tell a save file from another file. [`read()`]
//! loads one whole file.

use std::fs::{self, File};
use std::io::Read as _;
use std::path::Path;

use crate::atomic;
use crate::error::{IoError, Operation};

/// The longest file [`read()`] loads: 64 MiB. A PR save file is under 1 MiB, so
/// the limit stops a wrong file from filling the memory.
pub const MAX_READ_LEN: u64 = 64 * 1024 * 1024;

/// One regular file of a folder: its name, size and first bytes.
///
/// `Debug` prints save content (`head`): never log it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FileHead {
    /// The file name, valid UTF-8.
    pub name: String,
    /// The size in bytes, from the metadata.
    pub len: u64,
    /// The first bytes: at most the requested length, fewer for a shorter file.
    pub head: Vec<u8>,
}

/// The regular files of a folder.
///
/// `Debug` prints save content (the `head` of each file) and personal paths
/// (`unreadable`): never log it.
#[derive(Debug)]
#[non_exhaustive]
pub struct FolderFiles {
    /// The readable files, sorted by name in byte order. The temporary files
    /// of [`atomic::write`] are left out.
    pub files: Vec<FileHead>,
    /// How many regular files have a name that is not UTF-8. The listing
    /// skips them and keeps no trace of the name.
    pub non_utf8: usize,
    /// One `Io` error with [`Operation::Read`] per file whose metadata or
    /// first bytes could not be read. A game can lock a file while it writes
    /// it, so the listing goes on.
    pub unreadable: Vec<IoError>,
}

/// Lists the regular files of `dir`, each with its first `head_len` bytes.
///
/// A symbolic link is not followed: a link, a folder and any other kind of
/// entry are left out without a trace, as in [`backup::list`](crate::backup::list).
/// A `head_len` of 0 reads no byte.
///
/// # Errors
///
/// Returns [`IoError::Io`] with [`Operation::ListDir`] when `dir` cannot be
/// listed (missing, not a folder, no right to read it). A file that fails to
/// read is not an error of the call: it lands in [`FolderFiles::unreadable`].
pub fn list(dir: &Path, head_len: usize) -> Result<FolderFiles, IoError> {
    let list_error = |source| IoError::Io {
        operation: Operation::ListDir,
        path: dir.to_path_buf(),
        source,
    };
    let mut listing = FolderFiles {
        files: Vec::new(),
        non_utf8: 0,
        unreadable: Vec::new(),
    };
    for entry in fs::read_dir(dir).map_err(list_error)? {
        let entry = entry.map_err(list_error)?;
        let path = entry.path();
        let read_error = |source| IoError::Io {
            operation: Operation::Read,
            path: path.clone(),
            source,
        };
        // `DirEntry::file_type` does not follow a symbolic link.
        match entry.file_type() {
            Ok(kind) if kind.is_file() => {}
            Ok(_) => continue,
            Err(source) => {
                listing.unreadable.push(read_error(source));
                continue;
            }
        }
        let Ok(name) = entry.file_name().into_string() else {
            listing.non_utf8 += 1;
            continue;
        };
        if atomic::is_temp_name(&name) {
            continue;
        }
        match head_of(&path, head_len) {
            Ok((len, head)) => listing.files.push(FileHead { name, len, head }),
            Err(source) => listing.unreadable.push(read_error(source)),
        }
    }
    listing.files.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    Ok(listing)
}

/// The size of the file at `path` and its first `head_len` bytes.
fn head_of(path: &Path, head_len: usize) -> std::io::Result<(u64, Vec<u8>)> {
    let len = fs::metadata(path)?.len();
    let mut head = Vec::new();
    if head_len > 0 {
        let wanted = u64::try_from(head_len).unwrap_or(u64::MAX);
        File::open(path)?.take(wanted).read_to_end(&mut head)?;
    }
    Ok((len, head))
}

/// Reads the whole file `name` of `dir`.
///
/// `name` must be a plain file name: not empty, not `.` or `..`, and free of
/// `/`, `\`, `:` and NUL, so that `dir.join(name)` stays in `dir` on Unix and
/// on Windows. The function adds no symbolic link check to what
/// [`File::open`] does: a link in `dir` is followed.
///
/// # Errors
///
/// - [`IoError::InvalidFileName`] when `name` breaks the rule above.
/// - [`IoError::FileTooLarge`] when the file is longer than [`MAX_READ_LEN`],
///   found from its metadata before any read, or from the read itself when the
///   file grows meanwhile.
/// - [`IoError::Io`] with [`Operation::Read`] when the file does not open or
///   does not read.
pub fn read(dir: &Path, name: &str) -> Result<Vec<u8>, IoError> {
    if !is_plain_file_name(name) {
        return Err(IoError::InvalidFileName { name: name.to_owned() });
    }
    let path = dir.join(name);
    let read_error = |source| IoError::Io {
        operation: Operation::Read,
        path: path.clone(),
        source,
    };
    let too_large = || IoError::FileTooLarge {
        path: path.clone(),
        limit: MAX_READ_LEN,
    };
    let file = File::open(&path).map_err(read_error)?;
    let len = file.metadata().map_err(read_error)?.len();
    if len > MAX_READ_LEN {
        return Err(too_large());
    }
    let mut bytes = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
    file.take(MAX_READ_LEN + 1).read_to_end(&mut bytes).map_err(read_error)?;
    if u64::try_from(bytes.len()).is_ok_and(|read| read > MAX_READ_LEN) {
        return Err(too_large());
    }
    Ok(bytes)
}

/// The rule of `neoffse_core::SaveFiles::push`, copied because this crate
/// depends on no workspace crate.
fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', ':', '\0'])
}
