//! `Registry::backups` and `Registry::restore`: the round trip, the undo and
//! the stale session.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::Path;

use common::{data_dir, gil, gil_value, moment, no_process, open_session, set, slot_name};
use neoffse::core::Game;
use neoffse::data::Language;
use neoffse::io::atomic::RealFileOps;
use neoffse::io::backup::{Backup, Moment, Restored};
use neoffse::io::dirs::DataDir;
use neoffse::{Error, Registry, Running, Session};

fn registry() -> Registry {
    Registry::new(Language::En)
}

/// An instant `seconds` after the save's, for a later backup folder name.
fn later(seconds: i64) -> Moment {
    Moment::new(1_700_000_000 + seconds, 0).unwrap()
}

/// A session that saved a gil edit: the folder, the data folder, the session
/// and the bytes before and after the save.
struct Saved {
    dir: tempfile::TempDir,
    _data_root: tempfile::TempDir,
    data: DataDir,
    session: Session,
    start: i64,
    before: Vec<u8>,
    after: Vec<u8>,
}

fn saved_session() -> Saved {
    let (dir, mut session) = open_session();
    let (data_root, data) = data_dir();
    let name = slot_name(2);
    let before = fs::read(dir.path().join(name)).unwrap();
    let start = gil_value(&mut session);
    session.edit(&set(&gil(), start + 7)).unwrap();
    session.save_with(&RealFileOps, no_process, moment(), &data, Running::Refuse).unwrap();
    let after = fs::read(dir.path().join(name)).unwrap();
    Saved {
        dir,
        _data_root: data_root,
        data,
        session,
        start,
        before,
        after,
    }
}

fn restore_at(saved: &Saved, backup: &Backup, at: Moment) -> Restored {
    registry()
        .restore_with(&RealFileOps, no_process, at, &saved.data, backup, saved.dir.path(), Running::Refuse)
        .unwrap()
}

fn only_backup(saved: &Saved) -> Backup {
    let listing = registry().backups(&saved.data, Game::Ff1Pr).unwrap();
    assert_eq!(listing.backups.len(), 1);
    listing.backups.into_iter().next().unwrap()
}

fn file(dir: &Path) -> Vec<u8> {
    fs::read(dir.join(slot_name(2))).unwrap()
}

#[test]
fn a_restore_puts_back_the_bytes_before_the_save() {
    let saved = saved_session();
    let backup = only_backup(&saved);
    assert_eq!(backup.manifest.reason, "before-save");

    let restored = restore_at(&saved, &backup, later(60));
    assert_eq!(file(saved.dir.path()), saved.before);
    assert_eq!(restored.created.len(), 0);
    let safety = restored.safety.expect("the file was present, so a safety backup exists");
    assert_eq!(safety.manifest.reason, "before-restore");
    assert_eq!(fs::read(safety.dir.join(slot_name(2))).unwrap(), saved.after);

    let listing = registry().backups(&saved.data, Game::Ff1Pr).unwrap();
    assert_eq!(listing.unreadable.len(), 0);
    let reasons: Vec<&str> = listing.backups.iter().map(|backup| backup.manifest.reason.as_str()).collect();
    assert_eq!(reasons, ["before-restore", "before-save"]);

    let mut reopened = registry().open(saved.dir.path(), slot_name(2), None).unwrap();
    assert_eq!(gil_value(&mut reopened), saved.start);
}

#[test]
fn restoring_the_safety_backup_undoes_the_restore() {
    let saved = saved_session();
    let backup = only_backup(&saved);
    let safety = restore_at(&saved, &backup, later(60)).safety.unwrap();
    assert_eq!(file(saved.dir.path()), saved.before);

    restore_at(&saved, &safety, later(120));
    assert_eq!(file(saved.dir.path()), saved.after);
    let mut reopened = registry().open(saved.dir.path(), slot_name(2), None).unwrap();
    assert_eq!(gil_value(&mut reopened), saved.start + 7);
}

#[test]
fn the_session_that_saved_is_stale_after_a_restore() {
    let mut saved = saved_session();
    let backup = only_backup(&saved);
    restore_at(&saved, &backup, later(60));

    saved.session.edit(&set(&gil(), saved.start + 99)).unwrap();
    let error = saved
        .session
        .save_with(&RealFileOps, no_process, later(120), &saved.data, Running::Allow)
        .unwrap_err();
    assert!(matches!(error, Error::ChangedOnDisk { .. }), "expected ChangedOnDisk, got {error:?}");
    assert_eq!(file(saved.dir.path()), saved.before);
}

#[test]
fn a_game_with_no_backup_folder_lists_nothing() {
    let (_data_root, data) = data_dir();
    let listing = registry().backups(&data, Game::Ff1Pr).unwrap();
    assert_eq!(listing.backups.len(), 0);
    assert_eq!(listing.unreadable.len(), 0);
    assert!(!data.backups_dir().exists());
}

#[test]
fn the_real_restore_runs_once_with_allow() {
    let saved = saved_session();
    let backup = only_backup(&saved);
    let restored = registry().restore(&saved.data, &backup, saved.dir.path(), Running::Allow).unwrap();
    assert!(restored.safety.is_some());
    assert_eq!(file(saved.dir.path()), saved.before);
}
