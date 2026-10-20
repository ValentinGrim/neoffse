//! Linux discovery: which candidates count as roots, how libraries are
//! read, and what discovery reports or ignores.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use common::discovery::{
    GAME_A, ID_1, ID_2, assert_nothing, documents_save, environment, library, library_block, list_libraries, paths, prefix, run, site, steam_origin, steam_root,
};
use neoffse_io::discovery::{Environment, SteamGame, discover};
use neoffse_io::error::{IoError, Operation};

const NATIVE: &str = ".local/share/Steam";
const FLATPAK: &str = ".var/app/com.valvesoftware.Steam/.local/share/Steam";
const SNAP: &str = "snap/steam/common/.local/share/Steam";

fn write_list(root: &Path, content: impl AsRef<[u8]>) -> PathBuf {
    let list = root.join("steamapps/libraryfolders.vdf");
    fs::write(&list, content).unwrap();
    list
}

#[test]
fn finds_a_prefix_in_the_native_root() {
    let site = site("finds_a_prefix_in_the_native_root");
    let root = steam_root(&site, NATIVE);
    list_libraries(&root, &[]);
    let save = prefix(&root, &GAME_A, ID_1);

    let found = run(&site, &[GAME_A]);

    assert!(found.skipped.is_empty());
    assert_eq!(found.folders.len(), 1);
    let folder = &found.folders[0];
    assert_eq!(folder.game, "test-a");
    assert_eq!(folder.app_id, 4_242_001);
    assert_eq!(folder.steam_id, ID_1);
    assert_eq!(folder.path, save);
    assert_eq!(steam_origin(folder), (root.clone(), root));
}

#[cfg(unix)]
#[test]
fn symlinked_roots_give_one_folder() {
    let site = linked_site("symlinked_roots_give_one_folder");
    let found = run(&site, &[GAME_A]);
    assert_eq!(found.folders.len(), 1);
    assert!(found.skipped.is_empty());
}

#[cfg(unix)]
#[test]
fn symlinked_roots_report_a_broken_list_once() {
    let site = linked_site("symlinked_roots_report_a_broken_list_once");
    write_list(&site.home.join(NATIVE), "garbage");

    let found = run(&site, &[GAME_A]);

    assert_eq!(found.skipped.len(), 1);
    assert!(matches!(found.skipped[0], IoError::LibraryListInvalid { .. }));
}

/// The native root, reached by `.steam/steam`, `.steam/root` and
/// `.steam/debian-installation` too, with one prefix.
#[cfg(unix)]
fn linked_site(name: &str) -> common::discovery::Site {
    let site = site(name);
    let root = steam_root(&site, NATIVE);
    list_libraries(&root, &[]);
    prefix(&root, &GAME_A, ID_1);
    fs::create_dir_all(site.home.join(".steam")).unwrap();
    for link in ["steam", "root", "debian-installation"] {
        std::os::unix::fs::symlink(&root, site.home.join(".steam").join(link)).unwrap();
    }
    site
}

#[test]
fn a_library_listed_by_two_roots_is_probed_once() {
    let site = site("a_library_listed_by_two_roots_is_probed_once");
    let native = steam_root(&site, NATIVE);
    let flatpak = steam_root(&site, FLATPAK);
    let shared = library(&site, "x");
    list_libraries(&native, &[&shared]);
    list_libraries(&flatpak, &[&shared]);
    let save = prefix(&shared, &GAME_A, ID_1);

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), [save]);
    assert_eq!(steam_origin(&found.folders[0]), (native, shared));
}

#[test]
fn flatpak_and_snap_roots_are_found() {
    let site = site("flatpak_and_snap_roots_are_found");
    let flatpak = steam_root(&site, FLATPAK);
    let snap = steam_root(&site, SNAP);
    let in_flatpak = prefix(&flatpak, &GAME_A, ID_1);
    let in_snap = prefix(&snap, &GAME_A, ID_2);

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), [in_flatpak, in_snap]);
    assert_eq!(steam_origin(&found.folders[0]).0, flatpak);
    assert_eq!(steam_origin(&found.folders[1]).0, snap);
}

#[cfg(unix)]
#[test]
fn a_flatpak_link_reaches_the_flatpak_root() {
    let site = site("a_flatpak_link_reaches_the_flatpak_root");
    let flatpak = steam_root(&site, FLATPAK);
    let save = prefix(&flatpak, &GAME_A, ID_1);
    let steam_link = site.home.join(".var/app/com.valvesoftware.Steam/.steam");
    fs::create_dir_all(&steam_link).unwrap();
    std::os::unix::fs::symlink(&flatpak, steam_link.join("steam")).unwrap();

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), [save]);
}

#[test]
fn a_custom_data_home_is_searched_and_the_default_still_is() {
    let site = site("a_custom_data_home_is_searched_and_the_default_still_is");
    let custom = steam_root(&site, "xdg/Steam");
    let default = steam_root(&site, NATIVE);
    let in_custom = prefix(&custom, &GAME_A, ID_1);
    let in_default = prefix(&default, &GAME_A, ID_2);
    let environment = Environment {
        data_home: Some(site.home.join("xdg")),
        ..environment(&site)
    };

    let found = discover(&environment, &[GAME_A]);

    assert_eq!(paths(&found), [in_custom, in_default]);
}

#[test]
fn a_candidate_without_steamapps_is_not_a_root() {
    let site = site("a_candidate_without_steamapps_is_not_a_root");
    let bare = site.home.join(".steam/root");
    documents_save(&bare.join("compatdata/4242001/pfx/drive_c/users/steamuser/Documents"), &GAME_A, ID_1);

    let found = run(&site, &[GAME_A]);

    assert_nothing(&found);
}

#[test]
fn absent_candidates_are_silent() {
    let site = site("absent_candidates_are_silent");
    let found = run(&site, &[GAME_A]);
    assert!(found.folders.is_empty() && found.skipped.is_empty());

    // A file where a root should be: its `steamapps` lookup says NotADirectory.
    fs::create_dir_all(site.home.join(".steam")).unwrap();
    fs::write(site.home.join(".steam/steam"), b"not a folder").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(site.home.join("nowhere"), site.home.join(".steam/root")).unwrap();

    let found = run(&site, &[GAME_A]);
    assert_nothing(&found);
}

#[test]
fn a_missing_library_list_leaves_the_root_probed() {
    let site = site("a_missing_library_list_leaves_the_root_probed");
    let root = steam_root(&site, NATIVE);
    let save = prefix(&root, &GAME_A, ID_1);

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), [save]);
    assert!(found.skipped.is_empty());
}

#[test]
fn a_broken_library_list_is_reported_and_the_root_still_works() {
    let site = site("a_broken_library_list_is_reported_and_the_root_still_works");
    let (root, other) = (steam_root(&site, NATIVE), library(&site, "x"));
    let (in_root, in_other) = (prefix(&root, &GAME_A, ID_1), prefix(&other, &GAME_A, ID_2));
    let named = other.to_string_lossy().into_owned();
    let cases: [(Vec<u8>, bool); 4] = [
        (b"garbage".to_vec(), true),
        (
            format!("\"libraryfolders\"\n{{\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{named}\"\n").into_bytes(),
            true,
        ),
        (
            format!("\"LibraryFolders\"\n{{\n\t\"1\"\n\t{{\n\t\t\"path\"\t\t\"{named}\"\n\t}}\n}}\n").into_bytes(),
            true,
        ),
        (vec![0xFF, 0xFE], false),
    ];
    for (content, invalid) in cases {
        let list = write_list(&root, content);

        let found = run(&site, &[GAME_A]);

        assert_eq!(paths(&found), std::slice::from_ref(&in_root), "only the root is probed");
        assert!(!paths(&found).contains(&in_other));
        assert_eq!(found.skipped.len(), 1);
        match &found.skipped[0] {
            IoError::LibraryListInvalid { path, .. } => assert!(invalid && *path == list),
            IoError::Io {
                operation: Operation::Read,
                path,
                source,
            } => {
                assert!(!invalid && *path == list && source.kind() == ErrorKind::InvalidData);
            }
            other => panic!("unexpected error {other:?}"),
        }
    }
}

#[test]
fn libraries_follow_the_numeric_order_of_their_keys() {
    let site = site("libraries_follow_the_numeric_order_of_their_keys");
    let root = steam_root(&site, NATIVE);
    let (two, ten) = (library(&site, "two"), library(&site, "ten"));
    let text = format!(
        "\"libraryfolders\"\n{{\n{}{}{}}}\n",
        library_block("0", &root),
        library_block("10", &ten),
        library_block("2", &two)
    );
    write_list(&root, text);
    let saves: Vec<PathBuf> = [&root, &two, &ten].iter().map(|library| prefix(library, &GAME_A, ID_1)).collect();

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), saves);
}

#[test]
fn an_unmounted_library_is_silent() {
    let site = site("an_unmounted_library_is_silent");
    let root = steam_root(&site, NATIVE);
    list_libraries(&root, &[&site.base.join("gone")]);
    let save = prefix(&root, &GAME_A, ID_1);

    let found = run(&site, &[GAME_A]);

    assert_eq!(paths(&found), [save]);
    assert!(found.skipped.is_empty());
}

#[cfg(unix)]
#[test]
fn an_unreadable_root_is_reported() {
    let site = site("an_unreadable_root_is_reported");
    let (native, flatpak) = (steam_root(&site, NATIVE), steam_root(&site, FLATPAK));
    let save = prefix(&native, &GAME_A, ID_1);
    prefix(&flatpak, &GAME_A, ID_2);
    if !common::discovery::lock(&flatpak) {
        return;
    }

    let found = run(&site, &[GAME_A]);

    common::discovery::unlock(&flatpak);
    assert_eq!(paths(&found), [save]);
    assert_eq!(found.skipped.len(), 1);
    let IoError::Io { operation, path, source } = &found.skipped[0] else {
        panic!("unexpected error {:?}", found.skipped[0]);
    };
    assert_eq!(*operation, Operation::Read);
    assert_eq!(*path, flatpak.join("steamapps"));
    assert_eq!(source.kind(), ErrorKind::PermissionDenied);
}

#[test]
fn no_home_finds_nothing() {
    let site = site("no_home_finds_nothing");
    let root = steam_root(&site, NATIVE);
    prefix(&root, &GAME_A, ID_1);
    let games: [SteamGame<'_>; 1] = [GAME_A];

    assert_nothing(&discover(&Environment::default(), &games));
    let no_home = Environment {
        home: None,
        ..environment(&site)
    };
    let found = discover(&no_home, &games);
    assert_nothing(&found);
}
