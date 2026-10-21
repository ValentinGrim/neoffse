//! Helpers for the install tests: the real FF descriptors and manifests.

use std::fs;
use std::path::{Path, PathBuf};

use neoffse_format_pr::games::GAMES;
use neoffse_io::discovery::{Discovery, SteamGame};

/// The descriptors of the six games, in `GAMES` order.
pub(crate) fn games() -> Vec<SteamGame<'static>> {
    GAMES
        .iter()
        .map(|game| SteamGame {
            id: game.id,
            app_id: game.app_id,
            folder: game.folder,
        })
        .collect()
}

/// The descriptor of game number `number` (1 to 6).
pub(crate) fn game(number: usize) -> SteamGame<'static> {
    games()[number - 1]
}

/// Writes `<library>/steamapps/appmanifest_<app ID>.acf` and returns the
/// install path `<library>/steamapps/common/<folder>` it implies. The folder
/// itself is not created: a download in progress has none.
pub(crate) fn manifest(library: &Path, game: &SteamGame<'_>) -> PathBuf {
    let steamapps = library.join("steamapps");
    fs::create_dir_all(&steamapps).unwrap();
    fs::write(steamapps.join(format!("appmanifest_{}.acf", game.app_id)), b"\"AppState\"\n{\n}\n").unwrap();
    steamapps.join("common").join(game.folder)
}

/// Asserts that discovery found no install. The message names the count only:
/// an `Install` holds a personal path.
pub(crate) fn assert_no_install(found: &Discovery) {
    assert!(found.installs.is_empty(), "found {} install(s)", found.installs.len());
}
