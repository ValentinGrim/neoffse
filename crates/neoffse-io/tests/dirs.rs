//! `DataDir`: path building, game id validation and the absence of file
//! access.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::path::{Path, PathBuf};

use neoffse_format_pr::games::GAMES;
use neoffse_io::dirs::DataDir;
use neoffse_io::error::IoError;

fn tmp_root() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
}

#[test]
fn at_keeps_the_root_verbatim() {
    let absolute = tmp_root().join("verbatim");
    assert_eq!(DataDir::at(&absolute).root(), absolute);
    assert_eq!(DataDir::at("rel/x").root(), Path::new("rel/x"));
}

#[test]
fn backups_dir_sits_under_the_root() {
    let root = tmp_root().join("backups-dir");
    assert_eq!(DataDir::at(&root).backups_dir(), root.join("backups"));
}

#[test]
fn game_backups_dir_joins_the_game_id() {
    let root = tmp_root().join("game-dir");
    let dir = DataDir::at(&root).game_backups_dir("ff1-pr").unwrap();
    assert_eq!(dir, root.join("backups").join("ff1-pr"));
}

#[test]
fn game_backups_dir_accepts_the_manifest_ids() {
    let data = DataDir::at(tmp_root().join("manifest-ids"));
    for id in ["ff1-pr", "ff2-pr", "ff3-pr", "ff4-pr", "ff5-pr", "ff6-pr", "ff1"] {
        assert!(data.game_backups_dir(id).is_ok(), "{id}");
    }
}

#[test]
fn game_backups_dir_rejects_ids_that_leave_the_folder() {
    let data = DataDir::at(tmp_root().join("bad-ids"));
    for id in ["", ".", "..", "../x", "a/b", "a\\b", "/abs", "C:", "ff1.pr", "FF1", "ff 1", "ff1\n", "ffé"] {
        match data.game_backups_dir(id) {
            Err(IoError::InvalidGameId { game }) => assert_eq!(game, id),
            other => panic!("{id:?} gave {other:?}"),
        }
    }
}

#[test]
fn data_dir_touches_no_file() {
    let root = tmp_root().join("io-foundation-no-io");
    let _ = std::fs::remove_dir_all(&root);
    let data = DataDir::at(&root);
    let _ = data.root();
    let _ = data.backups_dir();
    data.game_backups_dir("ff1-pr").unwrap();
    assert!(!root.exists());
}

#[test]
fn platform_dir_is_absolute_and_named_after_the_app() {
    match DataDir::platform() {
        Ok(data) => {
            assert!(data.root().is_absolute(), "{}", data.root().display());
            assert!(data.root().components().any(|c| c.as_os_str() == "neoffse"));
        }
        Err(error) => {
            assert!(matches!(error, IoError::NoDataDir), "{error}");
            let no_home = std::env::var_os("HOME").is_none_or(|home| home.is_empty());
            assert!(cfg!(unix) && no_home, "a home directory exists but platform() failed");
        }
    }
}

#[test]
fn data_dir_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<DataDir>();
}

#[test]
fn clone_and_equality_follow_the_root() {
    let a = DataDir::at(tmp_root().join("eq-a"));
    assert_eq!(a, DataDir::at(tmp_root().join("eq-a")));
    assert_eq!(a, a.clone());
    assert_ne!(a, DataDir::at(tmp_root().join("eq-b")));
}

#[test]
fn every_pr_game_id_names_a_backup_folder() {
    let root = tmp_root().join("pr-game-ids");
    let data = DataDir::at(&root);
    for game in GAMES {
        assert_eq!(data.game_backups_dir(game.id).unwrap(), root.join("backups").join(game.id), "{}", game.id);
    }
}

/// Runs the ignored `child` test of this file in a new process with `HOME`
/// set to `home` and no `XDG_DATA_HOME`. The child asserts; the parent checks
/// that it ran exactly one test and that it passed.
#[cfg(unix)]
fn run_child(child: &str, home: &str) {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", child, "--nocapture", "--test-threads=1"])
        .env("HOME", home)
        .env_remove("XDG_DATA_HOME")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "child failed: {stdout}{stderr}");
    assert!(stdout.contains("1 passed"), "child did not run one test: {stdout}");
}

#[cfg(unix)]
#[test]
fn platform_refuses_a_relative_home() {
    run_child("child_platform_with_a_relative_home", "relative/home");
}

#[cfg(unix)]
#[test]
#[ignore = "runs only as a child of platform_refuses_a_relative_home"]
fn child_platform_with_a_relative_home() {
    assert!(matches!(DataDir::platform(), Err(IoError::NoDataDir)));
}

#[cfg(unix)]
#[test]
fn platform_follows_an_absolute_home() {
    let home = tmp_root().join("platform-absolute-home");
    run_child("child_platform_with_an_absolute_home", home.to_str().unwrap());
}

#[cfg(unix)]
#[test]
#[ignore = "runs only as a child of platform_follows_an_absolute_home"]
fn child_platform_with_an_absolute_home() {
    let home = std::env::var("HOME").unwrap();
    let data = DataDir::platform().unwrap();
    assert_eq!(data.root(), Path::new(&home).join(".local/share/neoffse"));
}
