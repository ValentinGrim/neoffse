//! Steam roots and libraries, shared by the Linux and the Windows search.
//!
//! Source: `docs/formats/pixel-remaster.md#steam-roots-and-libraries`.

use std::fs;
use std::path::{Path, PathBuf};

use super::library_list::library_paths;
use super::{Discovery, Install, SteamGame, settle};
use crate::error::{IoError, Operation};

/// A canonical library and the canonical root that named it first.
pub(super) struct Library {
    pub(super) root: PathBuf,
    pub(super) path: PathBuf,
}

/// The canonical candidates that hold a `steamapps` folder, without duplicates.
///
/// Several candidates can be links to one folder. Each canonical path is
/// checked once, so a folder reached five ways reports its `steamapps`
/// failure once. A candidate that cannot be canonicalized reports its own
/// failure; the caller never repeats a spelling, so none reports twice.
pub(super) fn roots(candidates: &[PathBuf], skipped: &mut Vec<IoError>) -> Vec<PathBuf> {
    let mut tried: Vec<PathBuf> = Vec::new();
    let mut roots = Vec::new();
    for candidate in candidates {
        let Some(canonical) = settle(fs::canonicalize(candidate), Operation::Read, candidate, skipped) else {
            continue;
        };
        if tried.contains(&canonical) {
            continue;
        }
        tried.push(canonical.clone());
        let steamapps = canonical.join("steamapps");
        let status = settle(fs::metadata(&steamapps), Operation::Read, &steamapps, skipped);
        if status.is_some_and(|status| status.is_dir()) {
            roots.push(canonical);
        }
    }
    roots
}

/// For each root, the root itself, then the libraries its list names, each
/// canonical and unique across all roots. The first root to name a library
/// owns it.
pub(super) fn libraries(roots: Vec<PathBuf>, skipped: &mut Vec<IoError>) -> Vec<Library> {
    let mut libraries: Vec<Library> = Vec::new();
    for root in roots {
        add(&mut libraries, &root, root.clone());
        let list = root.join("steamapps").join("libraryfolders.vdf");
        let Some(text) = settle(fs::read_to_string(&list), Operation::Read, &list, skipped) else {
            continue;
        };
        match library_paths(&text) {
            Ok(paths) => {
                for path in paths {
                    // A relative path would resolve against the process's current
                    // folder, so discovery would walk a folder it was not pointed at.
                    if !path.is_absolute() {
                        skipped.push(IoError::LibraryListInvalid {
                            path: list.clone(),
                            source: "a library path is not absolute".into(),
                        });
                        continue;
                    }
                    if let Some(canonical) = settle(fs::canonicalize(&path), Operation::Read, &path, skipped) {
                        add(&mut libraries, &root, canonical);
                    }
                }
            }
            Err(source) => skipped.push(IoError::LibraryListInvalid { path: list, source }),
        }
    }
    libraries
}

fn add(libraries: &mut Vec<Library>, root: &Path, path: PathBuf) {
    if libraries.iter().all(|library| library.path != path) {
        libraries.push(Library {
            root: root.to_path_buf(),
            path,
        });
    }
}

/// Adds an [`Install`] to `found.installs` for each game whose
/// `steamapps/appmanifest_<app ID>.acf` is a regular file in a library, in
/// library then game order.
///
/// The file's existence is the whole rule: no manifest is parsed, and the
/// `installdir` it holds equals `SteamGame::folder` per the format document.
/// A path that an earlier library or root already returned is not returned
/// again.
pub(super) fn installs(libraries: &[Library], games: &[SteamGame<'_>], found: &mut Discovery) {
    for library in libraries {
        let steamapps = library.path.join("steamapps");
        for game in games {
            let manifest = steamapps.join(format!("appmanifest_{}.acf", game.app_id));
            let status = settle(fs::metadata(&manifest), Operation::Read, &manifest, &mut found.skipped);
            if !status.is_some_and(|status| status.is_file()) {
                continue;
            }
            let path = steamapps.join("common").join(game.folder);
            if found.installs.iter().any(|install| install.path == path) {
                continue;
            }
            found.installs.push(Install {
                game: game.id.to_owned(),
                app_id: game.app_id,
                path,
            });
        }
    }
}
