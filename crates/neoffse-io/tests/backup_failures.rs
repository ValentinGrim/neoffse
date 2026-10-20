//! Backups under failure: a failed call leaves no folder, and an earlier
//! backup stays as it was.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::error::Error as _;
use std::fs;
use std::io::ErrorKind;

use common::Step;
use common::backup::{BESTIARY, Corrupt, SLOT, Snoop, T0, World, at, game_dir, make, request, tree, world};
use neoffse_io::atomic::RealFileOps;
use neoffse_io::backup::create_with;
use neoffse_io::error::{IoError, Operation};

const STAMP: &str = "20261018-201403";

fn failed(w: &World, files: &[&str]) -> IoError {
    create_with(&RealFileOps, &w.data, &request(w, "ff1-pr", files), at(T0)).unwrap_err()
}

/// The parts of an `IoError::Io`.
fn io_parts(error: &IoError) -> (Operation, &std::path::Path, ErrorKind) {
    match error {
        IoError::Io { operation, path, source } => (*operation, path, source.kind()),
        other => panic!("not an Io error: {other:?}"),
    }
}

#[test]
fn missing_source_file_leaves_no_folder() {
    let w = world("missing_source_file_leaves_no_folder");
    let source_before = tree(&w.source);
    let error = failed(&w, &[SLOT, "missing"]);
    let (operation, path, kind) = io_parts(&error);
    assert_eq!(operation, Operation::Read);
    assert_eq!(path, w.source.join("missing"));
    assert_eq!(kind, ErrorKind::NotFound);
    assert!(tree(&game_dir(&w, "ff1-pr")).is_empty());
    assert_eq!(tree(&w.source), source_before);
}

#[test]
fn directory_as_source_file_fails_the_copy() {
    let w = world("directory_as_source_file_fails_the_copy");
    fs::create_dir(w.source.join("sub")).unwrap();
    let error = failed(&w, &[SLOT, "sub"]);
    let (operation, path, _) = io_parts(&error);
    let expected = if cfg!(unix) { Operation::Copy } else { Operation::Read };
    assert_eq!(operation, expected);
    assert_eq!(path, w.source.join("sub"));
    assert!(tree(&game_dir(&w, "ff1-pr")).is_empty());
}

#[test]
fn manifest_rename_failure_removes_the_folder() {
    let w = world("manifest_rename_failure_removes_the_folder");
    let (snoop, _log) = Snoop::new(&[Step::Rename], false);
    let files = [SLOT, BESTIARY];
    let error = create_with(&snoop, &w.data, &request(&w, "ff1-pr", &files), at(T0)).unwrap_err();
    let (operation, path, _) = io_parts(&error);
    assert_eq!(operation, Operation::Rename);
    assert_eq!(path, game_dir(&w, "ff1-pr").join(STAMP).join("backup.json"));
    assert!(tree(&game_dir(&w, "ff1-pr")).is_empty());
}

#[test]
fn manifest_temp_failure_removes_the_folder() {
    let w = world("manifest_temp_failure_removes_the_folder");
    let (snoop, _log) = Snoop::new(&[Step::CreateTemp], false);
    let files = [SLOT, BESTIARY];
    let error = create_with(&snoop, &w.data, &request(&w, "ff1-pr", &files), at(T0)).unwrap_err();
    let (operation, path, _) = io_parts(&error);
    assert_eq!(operation, Operation::CreateTemp);
    assert_eq!(path, game_dir(&w, "ff1-pr").join(STAMP));
    assert!(tree(&game_dir(&w, "ff1-pr")).is_empty());
}

#[test]
fn manifest_that_reads_back_wrong_fails_verification() {
    let w = world("manifest_that_reads_back_wrong_fails_verification");
    let (snoop, _log) = Snoop::new(&[], true);
    let files = [SLOT, BESTIARY];
    let error = create_with(&snoop, &w.data, &request(&w, "ff1-pr", &files), at(T0)).unwrap_err();
    let IoError::VerificationFailed { path, .. } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(path, &game_dir(&w, "ff1-pr").join(STAMP).join("backup.json"));
    assert!(error.source().is_some());
    assert!(tree(&game_dir(&w, "ff1-pr")).is_empty());
}

#[test]
fn manifest_that_parses_but_differs_fails_verification() {
    let w = world("manifest_that_parses_but_differs_fails_verification");
    let (snoop, _log) = Snoop::corrupting(&[], Corrupt::Field);
    let files = [SLOT, BESTIARY];
    let error = create_with(&snoop, &w.data, &request(&w, "ff1-pr", &files), at(T0)).unwrap_err();
    let IoError::VerificationFailed { path, .. } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(path, &game_dir(&w, "ff1-pr").join(STAMP).join("backup.json"));
    assert!(error.source().is_some());
    assert!(tree(&game_dir(&w, "ff1-pr")).is_empty());
}

#[test]
fn suffix_exhaustion_fails_without_touching_the_folders() {
    let w = world("suffix_exhaustion_fails_without_touching_the_folders");
    let game = game_dir(&w, "ff1-pr");
    fs::create_dir_all(game.join(STAMP)).unwrap();
    for n in 1..=99 {
        fs::create_dir_all(game.join(format!("{STAMP}-{n:02}"))).unwrap();
    }
    let before = tree(&game);
    assert_eq!(before.len(), 100);
    let error = failed(&w, &[SLOT]);
    let (operation, path, kind) = io_parts(&error);
    assert_eq!(operation, Operation::CreateDir);
    assert_eq!(path, game.join(format!("{STAMP}-99")));
    assert_eq!(kind, ErrorKind::AlreadyExists);
    assert_eq!(tree(&game), before);
}

#[test]
fn data_folder_that_is_a_file_fails_on_create_dir() {
    let w = world("data_folder_that_is_a_file_fails_on_create_dir");
    fs::write(w.data.root(), b"not a folder").unwrap();
    let error = failed(&w, &[SLOT]);
    let (operation, path, _) = io_parts(&error);
    assert_eq!(operation, Operation::CreateDir);
    assert_eq!(path, game_dir(&w, "ff1-pr"));
    assert_eq!(fs::read(w.data.root()).unwrap(), b"not a folder");
}

#[test]
fn a_failed_backup_leaves_earlier_backups_intact() {
    let w = world("a_failed_backup_leaves_earlier_backups_intact");
    make(&w, "ff1-pr", T0);
    let before = tree(&game_dir(&w, "ff1-pr"));
    assert!(!before.is_empty());
    let error = failed(&w, &[SLOT, "missing"]);
    assert!(matches!(
        error,
        IoError::Io {
            operation: Operation::Read,
            ..
        }
    ));
    assert_eq!(tree(&game_dir(&w, "ff1-pr")), before);
}
