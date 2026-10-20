//! Discovery rules that a plain tree does not show: library spellings, the
//! root as a library, one folder per path, and the order and reports of the
//! probe.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::discovery::{GAME_A, GAME_B, ID_1, ID_2, Site, documents_save, library, library_block, list_libraries, paths, prefix, run, site, steam_root};
#[cfg(unix)]
use common::discovery::{environment, steam_dir, steam_origin};
use neoffse_io::discovery::library_list::library_paths;
use neoffse_io::discovery::{Environment, SteamGame, discover};
#[cfg(unix)]
use neoffse_io::error::{IoError, Operation};

const NATIVE: &str = ".local/share/Steam";

#[test]
fn library_block_doubles_backslashes() {
    let block = library_block("1", Path::new(r"D:\Games\Steam"));
    assert!(block.contains(r#""D:\\Games\\Steam""#), "{block}");
    let list = format!("\"libraryfolders\"\n{{\n{block}}}\n");
    assert_eq!(library_paths(&list).unwrap(), [PathBuf::from(r"D:\Games\Steam")]);
}

// Unix only: a verbatim Windows path (`\\?\`) does not resolve `..`, and a
// link needs a privilege there.
#[cfg(unix)]
#[test]
fn a_library_spelled_two_other_ways_is_probed_once() {
    let site = site("a_library_spelled_two_other_ways_is_probed_once");
    let root = steam_root(&site, NATIVE);
    let x = library(&site, "x");
    library(&site, "y");
    let dotdot = site.base.join("libs/y/../x");
    let link = site.base.join("link-to-x");
    std::os::unix::fs::symlink(&x, &link).unwrap();
    list_libraries(&root, &[&x, &dotdot, &link]);
    let save = prefix(&x, &GAME_A, ID_1);

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), [save]);
    assert_eq!(steam_origin(&found.folders[0]), (root, x));
    assert!(found.skipped.is_empty());
}

#[test]
fn the_root_is_a_library_even_when_its_list_omits_it() {
    let site = site("the_root_is_a_library_even_when_its_list_omits_it");
    let root = steam_root(&site, NATIVE);
    let x = library(&site, "x");
    let list = format!("\"libraryfolders\"\n{{\n{}}}\n", library_block("1", &x));
    fs::write(root.join("steamapps/libraryfolders.vdf"), list).unwrap();
    let in_root = prefix(&root, &GAME_A, ID_1);
    let in_x = prefix(&x, &GAME_A, ID_2);

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), [in_root, in_x]);
}

#[test]
fn a_game_listed_twice_gives_each_folder_once() {
    let site = site("a_game_listed_twice_gives_each_folder_once");
    let root = steam_root(&site, NATIVE);
    list_libraries(&root, &[]);
    prefix(&root, &GAME_A, ID_1);

    assert_eq!(run(&site, &[GAME_A, GAME_A]).folders.len(), 1);

    let alias = SteamGame { id: "test-a2", ..GAME_A };
    let found = run(&site, &[GAME_A, alias]);
    assert_eq!(found.folders.len(), 1);
    assert_eq!(found.folders[0].game, "test-a");
}

#[test]
fn a_game_listed_twice_gives_each_documents_folder_once() {
    let site = site("a_game_listed_twice_gives_each_documents_folder_once");
    let documents = site.base.join("Documents");
    documents_save(&documents, &GAME_A, ID_1);
    let environment = Environment {
        documents: Some(documents),
        ..Environment::default()
    };

    let found = discover(&environment, &[GAME_A, GAME_A]);

    assert_eq!(found.folders.len(), 1);
    assert!(found.skipped.is_empty());
}

#[test]
fn distinct_games_keep_their_own_folders() {
    let site = site("distinct_games_keep_their_own_folders");
    let root = steam_root(&site, NATIVE);
    list_libraries(&root, &[]);
    let (save_a, save_b) = (prefix(&root, &GAME_A, ID_1), prefix(&root, &GAME_B, ID_1));

    let found = run(&site, &[GAME_A, GAME_B]);

    assert_eq!(paths(&found), [save_a, save_b]);
}

#[cfg(unix)]
#[test]
fn an_unreadable_home_reports_each_candidate_once() {
    let site = site("an_unreadable_home_reports_each_candidate_once");
    let root = steam_root(&site, NATIVE);
    if !common::discovery::lock(&site.home) {
        return;
    }

    let found = discover(&environment(&site), &[GAME_A]);

    common::discovery::unlock(&site.home);
    let reported: Vec<&Path> = found
        .skipped
        .iter()
        .map(|error| match error {
            IoError::Io { path, .. } => path.as_path(),
            other => panic!("unexpected error {other:?}"),
        })
        .collect();
    for (index, path) in reported.iter().enumerate() {
        assert!(!reported[index + 1..].contains(path), "reported twice: {path:?}");
    }
    assert_eq!(reported.iter().filter(|path| **path == root).count(), 1);
}

#[cfg(unix)]
#[test]
fn a_steam_id_link_loop_is_reported() {
    let site = site("a_steam_id_link_loop_is_reported");
    let root = steam_root(&site, NATIVE);
    list_libraries(&root, &[]);
    let documents = root
        .join("steamapps/compatdata")
        .join(GAME_A.app_id.to_string())
        .join("pfx/drive_c/users/steamuser/Documents");
    let steam = steam_dir(&documents, &GAME_A);
    fs::create_dir_all(&steam).unwrap();
    let looped = steam.join(ID_1);
    std::os::unix::fs::symlink(ID_1, &looped).unwrap();

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), Vec::<PathBuf>::new());
    assert_eq!(found.skipped.len(), 1);
    let IoError::Io { operation, path, .. } = &found.skipped[0] else {
        panic!("unexpected error {:?}", found.skipped[0]);
    };
    assert_eq!(*operation, Operation::Read);
    assert_eq!(*path, looped);
}

/// Creates `ids` in this order in one fresh site and returns the site and the
/// folders in creation order.
fn created_in_order(name: &str, ids: [&str; 2]) -> (Site, Vec<PathBuf>) {
    let site = site(name);
    let root = steam_root(&site, NATIVE);
    list_libraries(&root, &[]);
    let saves = ids.map(|id| prefix(&root, &GAME_A, id)).to_vec();
    (site, saves)
}

#[test]
fn steam_ids_come_back_ascending_whatever_the_creation_order() {
    let (first, first_saves) = created_in_order("steam_ids_ascending_a", [ID_1, ID_2]);
    let (second, second_saves) = created_in_order("steam_ids_ascending_b", [ID_2, ID_1]);

    assert_eq!(paths(&run(&first, &[GAME_A])), first_saves);
    let expected = [second_saves[1].clone(), second_saves[0].clone()];
    assert_eq!(paths(&run(&second, &[GAME_A])), expected);
}
