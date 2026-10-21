//! Linux discovery of installed games: an `appmanifest_<app ID>.acf` in a
//! library, in library order, deduplicated across roots.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::PathBuf;

use common::discovery::{ID_1, library, list_libraries, paths, prefix, run, site, steam_root};
use common::installs::{assert_no_install, game, manifest};

const NATIVE: &str = ".local/share/Steam";
const FLATPAK: &str = ".var/app/com.valvesoftware.Steam/.local/share/Steam";

#[test]
fn manifests_in_two_libraries_give_two_installs_in_library_order() {
    let site = site("installs_two_libraries");
    let root = steam_root(&site, NATIVE);
    let second = library(&site, "second");
    list_libraries(&root, &[&second]);
    let first_path = manifest(&root, &game(1));
    let second_path = manifest(&second, &game(3));

    let found = run(&site, &[game(3), game(1)]);

    assert!(found.skipped.is_empty());
    assert_eq!(paths(&found), Vec::<PathBuf>::new());
    let seen: Vec<_> = found
        .installs
        .iter()
        .map(|install| (install.game.as_str(), install.app_id, install.path.clone()))
        .collect();
    assert_eq!(seen, [("ff1-pr", game(1).app_id, first_path), ("ff3-pr", game(3).app_id, second_path)]);
}

#[test]
fn games_in_one_library_follow_the_order_given() {
    let site = site("installs_game_order");
    let root = steam_root(&site, NATIVE);
    list_libraries(&root, &[]);
    manifest(&root, &game(1));
    manifest(&root, &game(2));

    let found = run(&site, &[game(2), game(1)]);

    let ids: Vec<&str> = found.installs.iter().map(|install| install.game.as_str()).collect();
    assert_eq!(ids, ["ff2-pr", "ff1-pr"]);
}

#[test]
fn a_prefix_without_a_manifest_is_a_save_folder_and_no_install() {
    let site = site("installs_prefix_only");
    let root = steam_root(&site, NATIVE);
    list_libraries(&root, &[]);
    prefix(&root, &game(1), ID_1);

    let found = run(&site, &[game(1)]);

    assert_eq!(found.folders.len(), 1);
    assert_no_install(&found);
    assert!(found.skipped.is_empty());
}

#[test]
fn a_manifest_that_is_a_folder_gives_no_install() {
    let site = site("installs_manifest_folder");
    let root = steam_root(&site, NATIVE);
    list_libraries(&root, &[]);
    let name = format!("appmanifest_{}.acf", game(1).app_id);
    fs::create_dir(root.join("steamapps").join(name)).unwrap();

    let found = run(&site, &[game(1)]);

    assert_no_install(&found);
    assert!(found.skipped.is_empty());
}

#[test]
fn a_manifest_of_another_game_gives_no_install() {
    let site = site("installs_other_game");
    let root = steam_root(&site, NATIVE);
    list_libraries(&root, &[]);
    manifest(&root, &game(2));

    let found = run(&site, &[game(1)]);

    assert_no_install(&found);
}

#[test]
fn two_roots_naming_one_library_give_one_install() {
    let site = site("installs_shared_library");
    let native = steam_root(&site, NATIVE);
    let flatpak = steam_root(&site, FLATPAK);
    let shared = library(&site, "shared");
    list_libraries(&native, &[&shared]);
    list_libraries(&flatpak, &[&shared]);
    let path = manifest(&shared, &game(4));

    let found = run(&site, &[game(4)]);

    assert!(found.skipped.is_empty());
    assert_eq!(found.installs.len(), 1);
    assert_eq!(found.installs[0].path, path);
}

#[test]
fn each_root_keeps_the_install_of_its_own_library() {
    let site = site("installs_two_roots");
    let native = steam_root(&site, NATIVE);
    let flatpak = steam_root(&site, FLATPAK);
    list_libraries(&native, &[]);
    list_libraries(&flatpak, &[]);
    let native_path = manifest(&native, &game(1));
    let flatpak_path = manifest(&flatpak, &game(1));

    let found = run(&site, &[game(1)]);

    let paths: Vec<_> = found.installs.iter().map(|install| install.path.clone()).collect();
    assert_eq!(paths, [native_path, flatpak_path]);
}
