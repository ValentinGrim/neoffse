//! `Registry::restore`: the running-game check and the unchecked bytes.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::cell::{Cell, RefCell};
use std::fs;
use std::path::Path;

use common::{data_dir, folder_with, moment, no_process, slot_bytes, slot_name};
use neoffse::core::Game;
use neoffse::data::Language;
use neoffse::io::atomic::RealFileOps;
use neoffse::io::backup::{self, Backup, Request};
use neoffse::io::dirs::DataDir;
use neoffse::pr::games::GAMES;
use neoffse::{Error, Registry, Running};

const GAME_ID: &str = "ff1-pr";

fn registry() -> Registry {
    Registry::new(Language::En)
}

fn executable() -> &'static str {
    GAMES.iter().find(|row| row.id == GAME_ID).unwrap().executable
}

/// A backup of `bytes` under the slot name, then a valid slot in the folder.
fn backed_up(game: &str, bytes: &[u8]) -> (tempfile::TempDir, tempfile::TempDir, DataDir, Backup) {
    let name = slot_name(2);
    let dir = folder_with(&[(name, bytes)]);
    let (data_root, data) = data_dir();
    let request = Request {
        game,
        source_dir: dir.path(),
        files: &[name],
        save_type: "slot",
        reason: "test",
    };
    let backup = backup::create(&data, &request).unwrap();
    fs::write(dir.path().join(name), slot_bytes()).unwrap();
    (dir, data_root, data, backup)
}

fn file(dir: &Path) -> Vec<u8> {
    fs::read(dir.join(slot_name(2))).unwrap()
}

#[test]
fn a_broken_file_comes_back_as_it_was() {
    let broken = b"no format opens these bytes".as_slice();
    let (dir, _data_root, data, backup) = backed_up(GAME_ID, broken);
    assert!(registry().open(dir.path(), slot_name(2), None).is_ok());

    let restored = registry()
        .restore_with(&RealFileOps, no_process, moment(), &data, &backup, dir.path(), Running::Refuse)
        .unwrap();
    assert_eq!(file(dir.path()), broken);
    assert_eq!(fs::read(restored.safety.unwrap().dir.join(slot_name(2))).unwrap(), slot_bytes());
    assert!(matches!(registry().open(dir.path(), slot_name(2), None), Err(Error::NoFormat { .. })));
}

#[test]
fn refuse_stops_when_the_probe_finds_the_game() {
    let (dir, _data_root, data, backup) = backed_up(GAME_ID, b"older bytes");
    let before = file(dir.path());
    let seen = RefCell::new(Vec::new());
    let probe = |executables: &[&'static str]| {
        seen.borrow_mut().extend_from_slice(executables);
        vec![executable()]
    };

    let error = registry()
        .restore_with(&RealFileOps, probe, moment(), &data, &backup, dir.path(), Running::Refuse)
        .unwrap_err();
    let Error::GameRunning { executables } = error else {
        panic!("expected GameRunning, got {error:?}")
    };
    assert_eq!(executables, [executable()]);
    assert_eq!(*seen.borrow(), [executable()]);
    assert_eq!(file(dir.path()), before);
    let listing = registry().backups(&data, Game::Ff1Pr).unwrap();
    assert_eq!(listing.backups.len(), 1);
    assert_eq!(listing.backups[0].manifest.reason, "test");

    seen.borrow_mut().clear();
    registry()
        .restore_with(&RealFileOps, probe, moment(), &data, &backup, dir.path(), Running::Allow)
        .unwrap();
    assert!(seen.borrow().is_empty());
    assert_eq!(file(dir.path()), b"older bytes");
}

#[test]
fn a_game_with_no_executable_is_never_probed() {
    let (dir, _data_root, data, backup) = backed_up("custom-game", b"older bytes");
    let called = Cell::new(false);
    let probe = |_: &[&'static str]| {
        called.set(true);
        vec![executable()]
    };
    registry()
        .restore_with(&RealFileOps, probe, moment(), &data, &backup, dir.path(), Running::Refuse)
        .unwrap();
    assert!(!called.get());
    assert_eq!(file(dir.path()), b"older bytes");
}
