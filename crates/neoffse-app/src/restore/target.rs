//! The folder a restore writes into, and whether it holds the open file.

use std::fs;
use std::path::{Path, PathBuf};

use neoffse::io::backup::Manifest;

/// The folder the backup was taken from, or `None` when the manifest text is
/// empty or not an absolute path.
///
/// A relative path would resolve against the app's working directory, so it
/// counts as missing. The function checks no disk. `source_dir` is lossy
/// UTF-8, so a folder whose name was not UTF-8 ends as missing too.
#[must_use]
pub fn target_of(manifest: &Manifest) -> Option<PathBuf> {
    let path = PathBuf::from(&manifest.source_dir);
    (!manifest.source_dir.is_empty() && path.is_absolute()).then_some(path)
}

/// Whether two paths name one folder: equal paths, or two paths that both
/// canonicalize to the same result.
///
/// Canonicalizing resolves symbolic links and `..`. When either side does not
/// canonicalize (a folder that does not exist), the plain comparison decides.
/// A wrong `false` leaves a stale session, whose next save ends in the
/// changed-on-disk dialog; a wrong `true` reopens a session that was not
/// stale.
#[must_use]
pub fn same_folder(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Whether restoring `files` into `target` replaces the file the open session
/// holds: its name is in `files` (exact, case-sensitive) and its folder is the
/// target.
#[must_use]
pub fn replaces_open_file(open_dir: &Path, open_name: &str, target: &Path, files: &[String]) -> bool {
    files.iter().any(|name| name == open_name) && same_folder(open_dir, target)
}
