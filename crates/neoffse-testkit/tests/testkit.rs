//! Fixture paths, saves root resolution, folder listing and the skip path.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::ffi::OsStr;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use neoffse_testkit::{SAVES_ENV, fixture_path, list_save_files, private_saves, read_fixture, resolve_saves_root, workspace_root};

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

/// Environment variable that names the game a child probe asks for.
const PROBE_GAME: &str = "NEOFFSE_TESTKIT_PROBE_GAME";

fn probe_game() -> String {
    std::env::var(PROBE_GAME).expect("runs only as a child of the skip tests")
}

/// Child of `run_probe`: `NEOFFSE_SAVES` points at a folder with no save.
#[test]
#[ignore = "runs only as a child of the skip tests"]
fn child_probe_private_saves() {
    assert_eq!(private_saves(&probe_game()), None);
}

/// Child of `run_probe`: `NEOFFSE_SAVES` points at a folder with one save.
#[test]
#[ignore = "runs only as a child of the skip tests"]
fn child_probe_private_saves_lists() {
    let files = private_saves(&probe_game()).expect("the game folder holds a file");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name(), Some(OsStr::new("a.sav")));
}

/// Runs this test binary again on one ignored probe, with `NEOFFSE_SAVES`
/// set to `root`. Editing the variable in this process would need `unsafe`.
fn run_probe(root: &Path, game: &str, probe: &str) -> Output {
    Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", probe, "--nocapture", "--test-threads=1"])
        .env(SAVES_ENV, root)
        .env(PROBE_GAME, game)
        .output()
        .unwrap()
}

/// Asserts that the child ran exactly one test and passed; returns its stderr.
/// A renamed probe or a bad filter runs zero tests and still exits with success.
fn child_stderr(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "child failed: {stderr}");
    assert!(stdout.contains("1 passed"), "child did not run one test: {stdout}");
    stderr
}

fn assert_skipped(output: &Output, expected: &[&str]) {
    let stderr = child_stderr(output);
    for part in expected {
        assert!(stderr.contains(part), "stderr lacks {part:?}: {stderr}");
    }
}

#[test]
fn private_saves_skips_an_empty_game_folder_with_a_reason() {
    let root = temp_dir("private_saves_skips_an_empty_game_folder_with_a_reason");
    std::fs::create_dir(root.join("ff9")).unwrap();
    let output = run_probe(&root, "ff9", "child_probe_private_saves");
    assert_skipped(&output, &["skipped: no private saves for ff9", "no file"]);
}

#[test]
fn private_saves_skips_a_missing_game_folder_with_a_reason() {
    let root = temp_dir("private_saves_skips_a_missing_game_folder_with_a_reason");
    let output = run_probe(&root, "ff9", "child_probe_private_saves");
    assert_skipped(&output, &["skipped: no private saves for ff9"]);
}

#[test]
fn private_saves_lists_a_folder_that_holds_a_file() {
    let root = temp_dir("private_saves_lists_a_folder_that_holds_a_file");
    std::fs::create_dir(root.join("ff9")).unwrap();
    std::fs::write(root.join("ff9").join("a.sav"), b"x").unwrap();
    let output = run_probe(&root, "ff9", "child_probe_private_saves_lists");
    let stderr = child_stderr(&output);
    assert!(!stderr.contains("skipped:"), "the folder holds a file, yet: {stderr}");
}
