//! `DataDir`: path building, game id validation and the absence of file
//! access.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::path::{Path, PathBuf};

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
