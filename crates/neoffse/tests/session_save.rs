//! `Session::save`: the pipeline, the clean point and the refusals.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::cell::RefCell;
use std::fs;

use common::{assert_gil_only_change, data_dir, gil, gil_value, level, moment, no_process, open_session, set, slot_name};
use neoffse::io::atomic::RealFileOps;
use neoffse::io::backup;
use neoffse::pr::games::GAMES;
use neoffse::{Error, Registry, Running};

const GAME_ID: &str = "ff1-pr";

#[test]
fn a_clean_session_saves_nothing() {
    let (dir, mut session) = open_session();
    let (data_root, data) = data_dir();
    let before = fs::read(dir.path().join(slot_name(2))).unwrap();
    let saved = session.save_with(&RealFileOps, no_process, moment(), &data, Running::Refuse).unwrap();
    assert!(saved.backup.is_none());
    assert_eq!(fs::read(dir.path().join(slot_name(2))).unwrap(), before);
    assert_eq!(fs::read_dir(data_root.path()).unwrap().count(), 0);
    assert!(!data.backups_dir().exists());
}

#[test]
fn a_save_backs_up_writes_and_cleans_the_session() {
    let (dir, mut session) = open_session();
    let (_data_root, data) = data_dir();
    let name = slot_name(2);
    let before = fs::read(dir.path().join(name)).unwrap();
    let gil_before = gil_value(&mut session);
    session.edit(&set(&gil(), gil_before + 7)).unwrap();

    let saved = session.save_with(&RealFileOps, no_process, moment(), &data, Running::Refuse).unwrap();
    let made = saved.backup.expect("a dirty save takes a backup");
    assert_eq!(made.manifest.reason, "before-save");
    assert_eq!(made.manifest.save_type, "slot");
    assert_eq!(made.manifest.game, GAME_ID);
    assert_eq!(made.manifest.files, [name]);
    assert_eq!(fs::read(made.dir.join(name)).unwrap(), before);
    let listing = backup::list(&data, GAME_ID).unwrap();
    assert_eq!(listing.backups, [made]);

    assert!(!session.is_dirty());
    assert_eq!(session.pending_count(), 0);
    let on_disk = fs::read(dir.path().join(name)).unwrap();
    assert_ne!(on_disk, before);
    assert_eq!(session.original().get(name), Some(on_disk.as_slice()));

    let mut reopened = Registry::new(neoffse::data::Language::En).open(dir.path(), name, None).unwrap();
    assert_eq!(gil_value(&mut reopened), gil_before + 7);
}

#[test]
fn a_save_changes_the_gil_and_nothing_else() {
    let (dir, mut session) = open_session();
    let (_data_root, data) = data_dir();
    let name = slot_name(2);
    let gil_before = gil_value(&mut session);
    session.edit(&set(&gil(), gil_before + 3)).unwrap();
    session.edit(&set(&level(), 2)).unwrap();
    assert!(session.undo().unwrap());
    let before = session.original().clone();

    session.save_with(&RealFileOps, no_process, moment(), &data, Running::Allow).unwrap();
    assert_gil_only_change(&before, gil_before + 3, dir.path(), name);
}

#[test]
fn a_save_moves_the_clean_point_inside_the_history() {
    let (_dir, mut session) = open_session();
    let (_data_root, data) = data_dir();
    let start = gil_value(&mut session);
    let save = |session: &mut neoffse::Session| {
        session.save_with(&RealFileOps, no_process, moment(), &data, Running::Allow).unwrap();
    };

    session.edit(&set(&gil(), start + 1)).unwrap();
    save(&mut session);
    assert!(session.undo().unwrap());
    assert_eq!(session.pending_count(), 1);
    assert!(session.is_dirty());

    // A fork below the moved clean point drops the redo branch that held it.
    session.edit(&set(&gil(), start + 2)).unwrap();
    assert_eq!(session.pending_count(), 2);
    assert!(session.undo().unwrap());
    assert_eq!(session.pending_count(), 1);
    assert!(session.is_dirty());

    session.edit(&set(&gil(), start + 3)).unwrap();
    save(&mut session);
    assert!(!session.is_dirty());
    assert!(session.undo().unwrap());
    assert!(session.is_dirty());
    assert!(session.redo().unwrap());
    assert!(!session.is_dirty());
}

#[test]
fn refuse_stops_when_the_probe_finds_the_game() {
    let (dir, mut session) = open_session();
    let (data_root, data) = data_dir();
    let name = slot_name(2);
    let before = fs::read(dir.path().join(name)).unwrap();
    let start = gil_value(&mut session);
    session.edit(&set(&gil(), start + 1)).unwrap();

    let executable = GAMES.iter().find(|row| row.id == GAME_ID).unwrap().executable;
    let seen = RefCell::new(Vec::new());
    let probe = |executables: &[&'static str]| {
        seen.borrow_mut().extend_from_slice(executables);
        vec![executable]
    };
    let error = session.save_with(&RealFileOps, probe, moment(), &data, Running::Refuse).unwrap_err();
    let Error::GameRunning { executables } = error else {
        panic!("expected GameRunning, got {error:?}")
    };
    assert_eq!(executables, [executable]);
    assert_eq!(*seen.borrow(), [executable]);
    assert_eq!(fs::read(dir.path().join(name)).unwrap(), before);
    assert_eq!(fs::read_dir(data_root.path()).unwrap().count(), 0);
    assert!(session.is_dirty());

    seen.borrow_mut().clear();
    session.save_with(&RealFileOps, probe, moment(), &data, Running::Allow).unwrap();
    assert!(seen.borrow().is_empty());
    assert!(!session.is_dirty());
}

#[test]
fn a_file_changed_on_disk_refuses_the_save() {
    let (dir, mut session) = open_session();
    let (data_root, data) = data_dir();
    let name = slot_name(2);
    fs::write(dir.path().join(name), b"written by another program").unwrap();
    let start = gil_value(&mut session);
    session.edit(&set(&gil(), start + 1)).unwrap();

    let error = session.save_with(&RealFileOps, no_process, moment(), &data, Running::Allow).unwrap_err();
    let Error::ChangedOnDisk { name: reported } = error else {
        panic!("expected ChangedOnDisk, got {error:?}")
    };
    assert_eq!(reported, name);
    assert_eq!(fs::read(dir.path().join(name)).unwrap(), b"written by another program");
    assert_eq!(fs::read_dir(data_root.path()).unwrap().count(), 0);
    assert!(session.is_dirty());
}

#[test]
fn a_file_deleted_on_disk_is_an_io_error() {
    let (dir, mut session) = open_session();
    let (_data_root, data) = data_dir();
    fs::remove_file(dir.path().join(slot_name(2))).unwrap();
    let start = gil_value(&mut session);
    session.edit(&set(&gil(), start + 1)).unwrap();
    let error = session.save_with(&RealFileOps, no_process, moment(), &data, Running::Allow).unwrap_err();
    assert!(matches!(error, Error::Io { .. }), "expected Io, got {error:?}");
    assert!(session.is_dirty());
}

#[test]
fn discard_after_a_save_keeps_the_saved_gil() {
    let (_dir, mut session) = open_session();
    let (_data_root, data) = data_dir();
    let start = gil_value(&mut session);
    session.edit(&set(&gil(), start + 5)).unwrap();
    session.save_with(&RealFileOps, no_process, moment(), &data, Running::Allow).unwrap();
    session.edit(&set(&gil(), start + 9)).unwrap();
    session.discard().unwrap();
    assert!(!session.is_dirty());
    assert_eq!(gil_value(&mut session), start + 5);
}

#[test]
fn the_real_save_runs_once_with_allow() {
    let (dir, mut session) = open_session();
    let (_data_root, data) = data_dir();
    let start = gil_value(&mut session);
    session.edit(&set(&gil(), start + 2)).unwrap();
    let saved = session.save(&data, Running::Allow).unwrap();
    assert!(saved.backup.is_some());
    assert!(!session.is_dirty());
    let mut reopened = Registry::new(neoffse::data::Language::En).open(dir.path(), slot_name(2), None).unwrap();
    assert_eq!(gil_value(&mut reopened), start + 2);
}
