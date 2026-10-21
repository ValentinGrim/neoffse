//! A synthetic machine under a temporary folder: a home with Steam, a
//! Documents folder. No helper reads the real environment.

use std::fs;
use std::path::PathBuf;

use neoffse::io::discovery::Environment;
use neoffse::pr::games::GAMES;
use tempfile::TempDir;

/// A synthetic `SteamID64`.
pub(crate) const ID_1: &str = "76560000000000001";
/// A second synthetic `SteamID64`, greater than `ID_1`.
pub(crate) const ID_2: &str = "76560000000000002";

/// A temporary machine. Dropping it deletes everything.
pub(crate) struct Site {
    dir: TempDir,
}

impl Site {
    pub(crate) fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("home")).unwrap();
        fs::create_dir_all(dir.path().join("Documents")).unwrap();
        Self { dir }
    }

    pub(crate) fn root(&self) -> PathBuf {
        self.dir.path().to_path_buf()
    }

    fn home(&self) -> PathBuf {
        self.dir.path().join("home")
    }
}

pub(crate) fn environment(site: &Site) -> Environment {
    Environment {
        home: Some(site.home()),
        data_home: Some(site.home().join(".local/share")),
        documents: Some(site.dir.path().join("Documents")),
        ..Environment::default()
    }
}

/// The Proton save folder of the game at `game_index` in `GAMES`, created.
pub(crate) fn steam_folder(site: &Site, game_index: usize, steam_id: &str) -> PathBuf {
    let game = &GAMES[game_index];
    let folder = site
        .home()
        .join(".local/share/Steam/steamapps/compatdata")
        .join(game.app_id.to_string())
        .join("pfx/drive_c/users/steamuser/Documents/My Games")
        .join(game.folder)
        .join("Steam")
        .join(steam_id);
    fs::create_dir_all(&folder).unwrap();
    folder
}

/// Writes one file in a Proton save folder and returns the folder.
pub(crate) fn steam_slot(site: &Site, game_index: usize, steam_id: &str, file_name: &str, bytes: &[u8]) -> PathBuf {
    let folder = steam_folder(site, game_index, steam_id);
    fs::write(folder.join(file_name), bytes).unwrap();
    folder
}

/// The Documents save folder of a game, created.
pub(crate) fn documents_folder(site: &Site, game_index: usize, steam_id: &str) -> PathBuf {
    let folder = site
        .dir
        .path()
        .join("Documents/My Games")
        .join(GAMES[game_index].folder)
        .join("Steam")
        .join(steam_id);
    fs::create_dir_all(&folder).unwrap();
    folder
}

/// Writes one file in a Documents save folder and returns the folder.
pub(crate) fn documents_slot(site: &Site, game_index: usize, steam_id: &str, file_name: &str, bytes: &[u8]) -> PathBuf {
    let folder = documents_folder(site, game_index, steam_id);
    fs::write(folder.join(file_name), bytes).unwrap();
    folder
}

/// Marks the game at `game_index` as installed in the home's Steam library.
pub(crate) fn install(site: &Site, game_index: usize) {
    let game = &GAMES[game_index];
    let steamapps = site.home().join(".local/share/Steam/steamapps");
    fs::create_dir_all(steamapps.join("common").join(game.folder)).unwrap();
    fs::write(steamapps.join(format!("appmanifest_{}.acf", game.app_id)), "\"AppState\"\n{\n}\n").unwrap();
}
