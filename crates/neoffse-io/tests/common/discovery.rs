//! Helpers that the `discovery_*` test crates share. Every tree sits under
//! the Cargo target temporary folder; no helper reads the real environment.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use neoffse_io::discovery::{Discovery, Environment, Origin, SaveFolder, SteamGame, discover};

/// A synthetic `SteamID64` (17 digits, starts `7656`).
pub(crate) const ID_1: &str = "76560000000000001";
/// A second synthetic `SteamID64`, greater than `ID_1`.
pub(crate) const ID_2: &str = "76560000000000002";

/// A fake game with its own app ID.
pub(crate) const GAME_A: SteamGame<'static> = SteamGame {
    id: "test-a",
    app_id: 4_242_001,
    folder: "TEST GAME A PR",
};
/// A second fake game.
pub(crate) const GAME_B: SteamGame<'static> = SteamGame {
    id: "test-b",
    app_id: 4_242_002,
    folder: "TEST GAME B PR",
};

/// A fresh test tree. `base` is canonical, so a test builds the expected
/// canonical path by joining names to it.
pub(crate) struct Site {
    pub(crate) base: PathBuf,
    pub(crate) home: PathBuf,
}

/// Builds `discovery-<name>` under the Cargo target temporary folder,
/// emptied first, with an empty `home` inside.
pub(crate) fn site(name: &str) -> Site {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("discovery-{name}"));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(dir.join("home")).unwrap();
    let base = fs::canonicalize(&dir).unwrap();
    Site { home: base.join("home"), base }
}

/// `<home>/<relative>` with a `steamapps` folder inside.
pub(crate) fn steam_root(site: &Site, relative: &str) -> PathBuf {
    let root = site.home.join(relative);
    fs::create_dir_all(root.join("steamapps")).unwrap();
    root
}

/// `<base>/libs/<name>` with a `steamapps` folder inside.
pub(crate) fn library(site: &Site, name: &str) -> PathBuf {
    let library = site.base.join("libs").join(name);
    fs::create_dir_all(library.join("steamapps")).unwrap();
    library
}

/// Writes `<root>/steamapps/libraryfolders.vdf` in the real shape: key `0`
/// is the root, then `1`, `2`, ... are `libraries`. A backslash doubles, as
/// Steam writes a Windows path.
pub(crate) fn list_libraries(root: &Path, libraries: &[&Path]) {
    let mut text = String::from("\"libraryfolders\"\n{\n");
    for (key, path) in std::iter::once(root).chain(libraries.iter().copied()).enumerate() {
        let path = path.to_string_lossy().replace('\\', "\\\\");
        write!(
            text,
            "\t\"{key}\"\n\t{{\n\t\t\"path\"\t\t\"{path}\"\n\t\t\"label\"\t\t\"\"\n\t\t\"apps\"\n\t\t{{\n\t\t}}\n\t}}\n"
        )
        .unwrap();
    }
    text.push_str("}\n");
    fs::write(root.join("steamapps").join("libraryfolders.vdf"), text).unwrap();
}

/// Creates the Proton save folder of `game` for `steam_id` in `library`,
/// with one file inside, and returns it.
pub(crate) fn prefix(library: &Path, game: &SteamGame<'_>, steam_id: &str) -> PathBuf {
    let documents = library
        .join("steamapps/compatdata")
        .join(game.app_id.to_string())
        .join("pfx/drive_c/users/steamuser/Documents");
    documents_save(&documents, game, steam_id)
}

/// Creates `<documents>/My Games/<folder>/Steam/<steam_id>` with one file
/// inside, and returns it.
pub(crate) fn documents_save(documents: &Path, game: &SteamGame<'_>, steam_id: &str) -> PathBuf {
    let save = steam_dir(documents, game).join(steam_id);
    fs::create_dir_all(&save).unwrap();
    fs::write(save.join("slot"), b"synthetic").unwrap();
    save
}

/// `<documents>/My Games/<folder>/Steam`.
pub(crate) fn steam_dir(documents: &Path, game: &SteamGame<'_>) -> PathBuf {
    documents.join("My Games").join(game.folder).join("Steam")
}

/// The Linux fields only: `home` and `<home>/.local/share` as `data_home`.
pub(crate) fn environment(site: &Site) -> Environment {
    Environment {
        home: Some(site.home.clone()),
        data_home: Some(site.home.join(".local/share")),
        ..Environment::default()
    }
}

/// Runs discovery on the Linux fields of `site`.
pub(crate) fn run(site: &Site, games: &[SteamGame<'_>]) -> Discovery {
    discover(&environment(site), games)
}

/// Asserts that discovery found nothing and skipped nothing.
pub(crate) fn assert_nothing(found: &Discovery) {
    assert!(found.folders.is_empty() && found.skipped.is_empty(), "found {found:?}");
}

/// The `path` of every folder, in order.
pub(crate) fn paths(found: &Discovery) -> Vec<PathBuf> {
    found.folders.iter().map(|folder| folder.path.clone()).collect()
}

/// The root and library of a Steam-library folder.
pub(crate) fn steam_origin(folder: &SaveFolder) -> (PathBuf, PathBuf) {
    match &folder.origin {
        Origin::SteamLibrary { root, library } => (root.clone(), library.clone()),
        other => panic!("expected a Steam library origin, found {other:?}"),
    }
}

/// Sets mode `0o000` on `path` and proves it blocks `read_dir`. Returns
/// `false`, with the mode restored, when it does not (the test runs as root).
#[cfg(unix)]
pub(crate) fn lock(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read_dir(path).is_ok() {
        unlock(path);
        return false;
    }
    true
}

/// Restores mode `0o755` on a folder that `lock` closed.
#[cfg(unix)]
pub(crate) fn unlock(path: &Path) {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
