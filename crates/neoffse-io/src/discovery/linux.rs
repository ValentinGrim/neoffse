//! The Linux search: Steam roots, their libraries, and the Proton prefixes.

use std::fs;
use std::path::{Path, PathBuf};

use super::library_list::library_paths;
use super::{Discovery, Environment, Origin, SteamGame, probe, settle};
use crate::error::{IoError, Operation};

/// A canonical library and the canonical root that named it first.
struct Library {
    root: PathBuf,
    path: PathBuf,
}

/// Probes every library of every root for every game.
///
/// Source: `docs/formats/pixel-remaster.md#steam-roots-and-libraries`. The
/// search does not run without a home.
pub(super) fn search(environment: &Environment, games: &[SteamGame<'_>], found: &mut Discovery) {
    let Some(home) = environment.home.as_deref() else {
        return;
    };
    let roots = roots(&candidates(home, environment.data_home.as_deref()), &mut found.skipped);
    let libraries = libraries(roots, &mut found.skipped);

    // `pixel-remaster.md#steam-roots-and-libraries`, "Prefixes outside the
    // game's library": Steam can leave a prefix away from the library that
    // installs the game, or behind after an uninstall. Every library is
    // probed for every game, and every hit is returned.
    for library in libraries {
        let origin = Origin::SteamLibrary {
            root: library.root,
            library: library.path.clone(),
        };
        for game in games {
            let steam_dir = library
                .path
                .join("steamapps")
                .join("compatdata")
                .join(game.app_id.to_string())
                .join("pfx/drive_c/users/steamuser/Documents/My Games")
                .join(game.folder)
                .join("Steam");
            probe(&steam_dir, game, &origin, found);
        }
    }
}

/// The places a Steam root can sit, in search order.
///
/// Source: `pixel-remaster.md#steam-roots-and-libraries`. The format document
/// names `$XDG_DATA_HOME/Steam` "or" `~/.local/share/Steam`. Both are tried:
/// a duplicate costs nothing after the dedupe, and a user who set the variable
/// after installing Steam still gets found.
fn candidates(home: &Path, data_home: Option<&Path>) -> Vec<PathBuf> {
    let mut list = Vec::new();
    if let Some(data_home) = data_home {
        list.push(data_home.join("Steam"));
    }
    list.push(home.join(".local/share/Steam"));
    list.push(home.join(".steam/steam"));
    list.push(home.join(".steam/root"));
    list.push(home.join(".steam/debian-installation"));
    for sandbox in [".var/app/com.valvesoftware.Steam", "snap/steam/common"] {
        let base = home.join(sandbox);
        list.push(base.join(".local/share/Steam"));
        list.push(base.join(".steam/steam"));
        list.push(base.join(".steam/root"));
    }
    list
}

/// The canonical candidates that hold a `steamapps` folder, without duplicates.
///
/// Three native candidates are links to one folder. Each canonical path is
/// checked once, so a folder reached five ways reports its failures once.
fn roots(candidates: &[PathBuf], skipped: &mut Vec<IoError>) -> Vec<PathBuf> {
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
fn libraries(roots: Vec<PathBuf>, skipped: &mut Vec<IoError>) -> Vec<Library> {
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
