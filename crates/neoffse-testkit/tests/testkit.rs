//! Fixture paths, saves root resolution and folder listing.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use neoffse_testkit::{fixture_path, list_save_files, private_saves, read_fixture, resolve_saves_root, workspace_root};

/// A fresh, empty folder under the target tmp dir, named after the test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn workspace_root_holds_the_workspace_manifest() {
    let manifest = std::fs::read_to_string(workspace_root().join("Cargo.toml")).unwrap();
    assert!(manifest.contains("[workspace]"));
}

#[test]
fn fixture_path_points_under_fixtures() {
    assert_eq!(fixture_path("a/b.bin"), workspace_root().join("fixtures").join("a").join("b.bin"));
}

#[test]
fn read_fixture_reports_a_missing_file() {
    let error = read_fixture("no-such-file.bin").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[test]
fn saves_root_defaults_to_fixtures_private() {
    let root = Path::new("/work/space");
    assert_eq!(resolve_saves_root(None, root), root.join("fixtures").join("private"));
}

#[test]
fn empty_env_value_falls_back_to_the_default() {
    let root = Path::new("/work/space");
    assert_eq!(resolve_saves_root(Some(OsStr::new("")), root), root.join("fixtures").join("private"));
}

#[test]
fn absolute_env_value_is_used_as_is() {
    let root = Path::new("/work/space");
    let value = Path::new(env!("CARGO_TARGET_TMPDIR")).join("elsewhere");
    assert!(value.is_absolute());
    assert_eq!(resolve_saves_root(Some(value.as_os_str()), root), value);
}

#[test]
fn relative_env_value_resolves_from_the_workspace_root() {
    let root = Path::new("/work/space");
    assert_eq!(resolve_saves_root(Some(OsStr::new("x/y")), root), root.join("x/y"));
}

#[test]
fn list_save_files_sorts_files_and_skips_folders() {
    let dir = temp_dir("list_save_files_sorts_files_and_skips_folders");
    std::fs::write(dir.join("b"), b"b").unwrap();
    std::fs::write(dir.join("a"), b"a").unwrap();
    std::fs::create_dir(dir.join("c")).unwrap();
    assert_eq!(list_save_files(&dir).unwrap(), vec![dir.join("a"), dir.join("b")]);
}

#[test]
fn list_save_files_on_an_empty_folder_is_empty() {
    let dir = temp_dir("list_save_files_on_an_empty_folder_is_empty");
    assert_eq!(list_save_files(&dir).unwrap(), Vec::<PathBuf>::new());
}

#[test]
fn list_save_files_reports_a_missing_folder() {
    let dir = temp_dir("list_save_files_reports_a_missing_folder").join("absent");
    let error = list_save_files(&dir).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::NotFound);
}

#[test]
fn private_saves_of_an_unknown_game_is_none() {
    assert_eq!(private_saves("no-such-game-0"), None);
}
