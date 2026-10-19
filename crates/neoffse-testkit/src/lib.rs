//! Test helpers: locate and read the committed fixtures under `fixtures/`,
//! and locate the private real saves named by `NEOFFSE_SAVES`. The crate
//! generates no fixture. A test that finds no private save returns early
//! and passes, after [`skip`] has written the reason on stderr.
//!
//! A test that needs real saves starts with:
//!
//! ```text
//! let Some(files) = neoffse_testkit::private_saves("ff1") else { return };
//! ```

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

/// Name of the environment variable that points at the private saves.
pub const SAVES_ENV: &str = "NEOFFSE_SAVES";

/// Root folder of the workspace, independent of the current directory.
///
/// `cargo test` runs each crate from its own folder, so tests cannot use
/// relative paths.
#[must_use]
pub fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.ancestors().nth(2).unwrap_or(manifest_dir).to_path_buf()
}

/// Path of a committed fixture, relative to the `fixtures/` folder.
#[must_use]
pub fn fixture_path(relative: &str) -> PathBuf {
    workspace_root().join("fixtures").join(relative)
}

/// Reads a committed fixture, relative to the `fixtures/` folder.
///
/// # Errors
///
/// Returns the I/O error of [`std::fs::read`], for instance `NotFound`.
pub fn read_fixture(relative: &str) -> io::Result<Vec<u8>> {
    std::fs::read(fixture_path(relative))
}

/// Resolves the folder that holds the private saves.
///
/// `env_value` is the content of [`SAVES_ENV`]. `None` or an empty value
/// gives `<root>/fixtures/private`; an absolute path is used as is; a
/// relative path resolves from `workspace_root`.
#[must_use]
pub fn resolve_saves_root(env_value: Option<&OsStr>, workspace_root: &Path) -> PathBuf {
    match env_value {
        Some(value) if !value.is_empty() => workspace_root.join(value),
        _ => workspace_root.join("fixtures").join("private"),
    }
}

/// Lists the regular files directly inside `dir`, sorted by path.
///
/// Subfolders are left out and nothing is read recursively. An empty
/// folder gives an empty list.
///
/// # Errors
///
/// Returns the I/O error of [`std::fs::read_dir`] when `dir` cannot be
/// opened, for instance `NotFound`, or of a directory entry.
pub fn list_save_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Finds the private save files of one game, such as `"ff1"`.
///
/// Looks in `<saves root>/<game>`, where the saves root comes from
/// [`SAVES_ENV`] or defaults to `fixtures/private`. Returns `None` after
/// calling [`skip`] when the folder is missing, unreadable or empty.
#[must_use]
pub fn private_saves(game: &str) -> Option<Vec<PathBuf>> {
    let env_value = std::env::var_os(SAVES_ENV);
    let root = resolve_saves_root(env_value.as_deref(), &workspace_root());
    let dir = root.join(game);
    let cause = match list_save_files(&dir) {
        Ok(files) if !files.is_empty() => return Some(files),
        Ok(_) => "no file".to_owned(),
        Err(error) => error.to_string(),
    };
    skip(&format!("no private saves for {game}: {SAVES_ENV} folder {} ({cause})", dir.display()));
    None
}

/// Reports on stderr why a test returns early without failing.
///
/// `cargo test` shows the line with `--nocapture` or when the test fails.
#[allow(clippy::print_stderr, reason = "a skipped test must say why, and stderr is the only channel it has")]
pub fn skip(reason: &str) {
    eprintln!("skipped: {reason}");
}
