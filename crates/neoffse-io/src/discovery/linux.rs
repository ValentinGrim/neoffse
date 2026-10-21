//! The Linux search: Steam roots, their libraries, and the Proton prefixes.

use std::path::{Path, PathBuf};

use super::steam::{installs, libraries, roots};
use super::{Discovery, Environment, Origin, SteamGame, probe};

/// Probes every library of every root for every game, then lists the installs.
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
    for library in &libraries {
        let origin = Origin::SteamLibrary {
            root: library.root.clone(),
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
    installs(&libraries, games, found);
}

/// The places a Steam root can sit, in search order, each spelling once.
///
/// Source: `pixel-remaster.md#steam-roots-and-libraries`. The format document
/// names `$XDG_DATA_HOME/Steam` "or" `~/.local/share/Steam`. Both are tried:
/// a user who set the variable after installing Steam still gets found. When
/// the data home is the default, the two are one path, and the list keeps the
/// first. The comparison is exact equality, before any `canonicalize`.
fn candidates(home: &Path, data_home: Option<&Path>) -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> = Vec::new();
    let mut push = |path: PathBuf| {
        if !list.contains(&path) {
            list.push(path);
        }
    };
    if let Some(data_home) = data_home {
        push(data_home.join("Steam"));
    }
    push(home.join(".local/share/Steam"));
    push(home.join(".steam/steam"));
    push(home.join(".steam/root"));
    push(home.join(".steam/debian-installation"));
    for sandbox in [".var/app/com.valvesoftware.Steam", "snap/steam/common"] {
        let base = home.join(sandbox);
        push(base.join(".local/share/Steam"));
        push(base.join(".steam/steam"));
        push(base.join(".steam/root"));
    }
    list
}
