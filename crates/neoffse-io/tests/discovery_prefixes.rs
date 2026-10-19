//! Linux discovery: which prefixes and `SteamID64` folders count.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::Path;

use common::backup::tree;
use common::discovery::{GAME_A, GAME_B, ID_1, ID_2, assert_nothing, library, list_libraries, paths, prefix, run, site, steam_origin, steam_root};
use neoffse_io::discovery::{SteamGame, is_steam_id64};

const NATIVE: &str = ".local/share/Steam";

#[test]
fn a_prefix_in_another_library_than_the_game_is_found() {
    let site = site("a_prefix_in_another_library_than_the_game_is_found");
    let (root, other) = (steam_root(&site, NATIVE), library(&site, "b"));
    fs::write(root.join("steamapps/appmanifest_4242001.acf"), b"").unwrap();
    list_libraries(&root, &[&other]);
    let save = prefix(&other, &GAME_A, ID_1);

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), [save]);
    assert_eq!(steam_origin(&found.folders[0]), (root, other));
}

#[test]
fn a_prefix_without_an_installed_game_is_found() {
    let site = site("a_prefix_without_an_installed_game_is_found");
    let root = steam_root(&site, NATIVE);
    let save = prefix(&root, &GAME_A, ID_1);
    assert!(!root.join("steamapps/common").exists());

    assert_eq!(paths(&run(&site, &[GAME_A])), [save]);
}

#[test]
fn every_library_is_probed_and_every_hit_returned() {
    let site = site("every_library_is_probed_and_every_hit_returned");
    let root = steam_root(&site, NATIVE);
    let (one, two) = (library(&site, "one"), library(&site, "two"));
    list_libraries(&root, &[&one, &two]);
    let expected = vec![prefix(&one, &GAME_A, ID_1), prefix(&two, &GAME_B, ID_1), prefix(&two, &GAME_A, ID_1)];

    let found = run(&site, &[GAME_B, GAME_A]);

    assert_eq!(paths(&found), expected);
}

/// Names that look like a `SteamID64` and are not one.
fn rejected() -> Vec<String> {
    let arabic: String = (0..13).map(|digit| char::from_u32(0x660 + digit % 10).unwrap()).collect();
    vec![
        "7656000000000000".to_owned(),
        "765600000000000000".to_owned(),
        "76550000000000001".to_owned(),
        "7656000000000000a".to_owned(),
        format!("7656{arabic}"),
        "notes".to_owned(),
        String::new(),
        format!("{ID_1} "),
    ]
}

#[test]
fn only_steam_id64_folders_count() {
    let site = site("only_steam_id64_folders_count");
    let root = steam_root(&site, NATIVE);
    let first = prefix(&root, &GAME_A, ID_2);
    let steam = first.parent().unwrap();
    let second = prefix(&root, &GAME_A, ID_1);
    for name in rejected().iter().filter(|name| !name.is_empty() && !name.ends_with(' ')) {
        fs::create_dir(steam.join(name)).unwrap();
    }
    fs::write(steam.join("76560000000000003"), b"a file").unwrap();

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), [second, first]);
}

#[test]
fn is_steam_id64_checks_the_shape() {
    assert!(is_steam_id64(ID_1) && is_steam_id64(ID_2));
    for name in rejected() {
        assert!(!is_steam_id64(&name), "{name:?}");
    }
}

#[test]
fn a_game_that_is_not_asked_for_is_ignored() {
    let site = site("a_game_that_is_not_asked_for_is_ignored");
    let root = steam_root(&site, NATIVE);
    let other = SteamGame {
        id: "test-c",
        app_id: 4_242_003,
        folder: "TEST GAME C PR",
    };
    prefix(&root, &other, ID_1);

    assert_nothing(&run(&site, &[GAME_A]));
    assert_nothing(&run(&site, &[]));
}

#[cfg(unix)]
#[test]
fn a_symlinked_steam_id_folder_counts() {
    use common::discovery::steam_dir;

    let site = site("a_symlinked_steam_id_folder_counts");
    let root = steam_root(&site, NATIVE);
    let elsewhere = site.base.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    let steam = steam_dir(&root.join("steamapps/compatdata/4242001/pfx/drive_c/users/steamuser/Documents"), &GAME_A);
    fs::create_dir_all(&steam).unwrap();
    std::os::unix::fs::symlink(&elsewhere, steam.join(ID_1)).unwrap();
    std::os::unix::fs::symlink(site.base.join("nowhere"), steam.join(ID_2)).unwrap();

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), [steam.join(ID_1)]);
    assert!(found.skipped.is_empty());
}

#[test]
fn discovery_changes_nothing_on_disk() {
    let site = site("discovery_changes_nothing_on_disk");
    let (root, other) = (steam_root(&site, NATIVE), library(&site, "b"));
    list_libraries(&root, &[&other]);
    prefix(&root, &GAME_A, ID_1);
    prefix(&other, &GAME_B, ID_2);
    let before = tree(&site.base);

    let found = run(&site, &[GAME_A, GAME_B]);

    assert_eq!(found.folders.len(), 2);
    assert_eq!(tree(&site.base), before);
}

#[cfg(unix)]
#[test]
fn an_unreadable_prefix_is_reported() {
    use neoffse_io::error::{IoError, Operation};

    let site = site("an_unreadable_prefix_is_reported");
    let root = steam_root(&site, NATIVE);
    let locked_save = prefix(&root, &GAME_A, ID_1);
    let other = prefix(&root, &GAME_B, ID_1);
    let locked = root.join("steamapps/compatdata/4242001");
    if !common::discovery::lock(&locked) {
        return;
    }

    let found = run(&site, &[GAME_A, GAME_B]);

    common::discovery::unlock(&locked);
    assert_eq!(paths(&found), [other]);
    assert_eq!(found.skipped.len(), 1);
    let IoError::Io { operation, path, .. } = &found.skipped[0] else {
        panic!("unexpected error {:?}", found.skipped[0]);
    };
    assert_eq!(*operation, Operation::ListDir);
    assert_eq!(*path, locked_save.parent().unwrap());
}

#[test]
fn a_relative_library_path_is_skipped_and_reported() {
    use neoffse_io::error::IoError;

    let site = site("a_relative_library_path_is_skipped_and_reported");
    let (root, other) = (steam_root(&site, NATIVE), library(&site, "b"));
    list_libraries(&root, &[Path::new("relative-library"), &other]);
    let save = prefix(&other, &GAME_A, ID_1);

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), [save]);
    assert_eq!(found.skipped.len(), 1);
    let IoError::LibraryListInvalid { path, .. } = &found.skipped[0] else {
        panic!("unexpected error {:?}", found.skipped[0]);
    };
    assert_eq!(*path, root.join("steamapps/libraryfolders.vdf"));
}
