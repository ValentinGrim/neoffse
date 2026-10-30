//! Windows discovery of installed games, fed through `program_files_x86` so
//! that it runs on every system. The libraries add installs, never saves.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::{Component, Path, PathBuf};

use common::discovery::{ID_1, library, list_libraries, paths, prefix, run, site, steam_root};
use common::installs::{assert_no_install, game, manifest};
use neoffse_io::discovery::{Environment, discover};

fn windows(program_files: &Path) -> Environment {
    Environment {
        program_files_x86: Some(program_files.to_path_buf()),
        ..Environment::default()
    }
}

/// `<base>/pf/Steam` with a `steamapps` folder, and the `pf` folder.
fn windows_root(base: &Path) -> (PathBuf, PathBuf) {
    let program_files = base.join("pf");
    let root = program_files.join("Steam");
    fs::create_dir_all(root.join("steamapps")).unwrap();
    (program_files, root)
}

#[test]
fn a_manifest_in_the_steam_root_gives_an_install() {
    let site = site("windows_installs_root");
    let (program_files, root) = windows_root(&site.base);
    let path = manifest(&root, &game(2));

    let found = discover(&windows(&program_files), &[game(1), game(2)]);

    assert!(found.skipped.is_empty());
    assert_eq!(found.installs.len(), 1);
    assert_eq!(found.installs[0].game, "ff2-pr");
    assert_eq!(found.installs[0].app_id, game(2).app_id);
    assert_eq!(found.installs[0].path, path);
}

#[test]
fn a_second_library_adds_an_install_and_no_save_folder() {
    let site = site("windows_installs_second_library");
    let (program_files, root) = windows_root(&site.base);
    let second = library(&site, "second");
    list_libraries(&root, &[&second]);
    let first_path = manifest(&root, &game(2));
    let second_path = manifest(&second, &game(5));
    // A Proton-shaped prefix in a library gives no save folder on Windows.
    prefix(&second, &game(2), ID_1);

    let found = discover(&windows(&program_files), &[game(2), game(5)]);

    assert!(found.skipped.is_empty());
    assert_eq!(paths(&found), Vec::<PathBuf>::new());
    let paths: Vec<_> = found.installs.iter().map(|install| install.path.clone()).collect();
    assert_eq!(paths, [first_path, second_path]);
}

#[test]
fn a_steam_folder_without_steamapps_is_not_a_root() {
    let site = site("windows_installs_no_steamapps");
    let program_files = site.base.join("pf");
    fs::create_dir_all(program_files.join("Steam")).unwrap();

    let found = discover(&windows(&program_files), &[game(2)]);

    assert_no_install(&found);
    assert!(found.skipped.is_empty());
}

#[test]
fn an_absent_program_files_folder_is_silent() {
    let site = site("windows_installs_absent");

    let found = discover(&windows(&site.base.join("missing")), &[game(2)]);

    assert_no_install(&found);
    assert!(found.skipped.is_empty());
}

#[test]
fn linux_installs_come_before_windows_installs() {
    let site = site("windows_installs_order");
    let linux_root = steam_root(&site, ".local/share/Steam");
    list_libraries(&linux_root, &[]);
    let linux_path = manifest(&linux_root, &game(1));
    let (program_files, root) = windows_root(&site.base);
    let windows_path = manifest(&root, &game(1));
    let environment = Environment {
        program_files_x86: Some(program_files),
        ..common::discovery::environment(&site)
    };

    let found = discover(&environment, &[game(1)]);

    let paths: Vec<_> = found.installs.iter().map(|install| install.path.clone()).collect();
    assert_eq!(paths, [linux_path, windows_path]);
    assert_eq!(run(&site, &[game(1)]).installs.len(), 1);
}

fn drive(path: &Path) -> Option<Component<'_>> {
    path.components().find(|part| matches!(part, Component::Prefix(_)))
}

/// `target` spelled relative to the current folder, `None` when the two
/// sit on different drives.
fn relative(target: &Path) -> Option<PathBuf> {
    let current = fs::canonicalize(std::env::current_dir().unwrap()).unwrap();
    if drive(&current) != drive(target) {
        return None;
    }
    let names = |path: &Path| -> Vec<_> {
        path.components()
            .filter_map(|part| match part {
                Component::Normal(name) => Some(name.to_owned()),
                _ => None,
            })
            .collect()
    };
    let mut spelled: PathBuf = names(&current).iter().map(|_| "..").collect();
    spelled.extend(names(target));
    assert!(spelled.is_relative());
    Some(spelled)
}

#[test]
fn a_relative_program_files_folder_is_ignored_in_silence() {
    let site = site("windows_installs_relative");
    let (program_files, root) = windows_root(&site.base);
    manifest(&root, &game(2));
    let Some(spelled) = relative(&program_files) else {
        return;
    };
    assert!(spelled.join("Steam/steamapps").exists(), "the relative spelling must resolve");

    let control = discover(&windows(&program_files), &[game(2)]);
    let found = discover(&windows(&spelled), &[game(2)]);

    assert_eq!(control.installs.len(), 1);
    assert_no_install(&found);
    assert!(found.skipped.is_empty());
}
