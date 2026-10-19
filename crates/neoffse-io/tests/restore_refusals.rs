//! Restore, phase 1: every refusal happens before anything changes.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use common::backup::{BESTIARY, SLOT, T0, World, at, edit, make, manifest_text, restore_at, tree, world};
use neoffse_io::atomic::RealFileOps;
use neoffse_io::backup::{Backup, Restored, restore_with};
use neoffse_io::error::{IoError, Operation};

type Tree = BTreeMap<String, Vec<u8>>;

/// The save folder and the data folder, to compare before and after a refusal.
fn snapshot(w: &World) -> (Tree, Tree) {
    (tree(&w.source), tree(w.data.root()))
}

fn restore_into(w: &World, game: &str, backup: &Backup, target: &Path) -> Result<Restored, IoError> {
    restore_with(&RealFileOps, &w.data, game, backup, target, at(T0 + 60))
}

/// The parts of an `IoError::Io`.
fn io_parts(error: &IoError) -> (Operation, &Path, ErrorKind) {
    match error {
        IoError::Io { operation, path, source } => (*operation, path, source.kind()),
        other => panic!("not an Io error: {other:?}"),
    }
}

/// Asserts a `ManifestInvalid` error for `backup.json` of `backup`, and
/// returns the text of its source.
fn manifest_refusal(error: &IoError, backup: &Backup) -> String {
    let IoError::ManifestInvalid { path, source } = error else {
        panic!("not a ManifestInvalid error: {error:?}");
    };
    assert_eq!(path, &backup.dir.join("backup.json"));
    source.to_string()
}

#[test]
fn a_backup_of_another_game_is_refused() {
    let w = world("restore-a_backup_of_another_game_is_refused");
    let backup = make(&w, "ff2-pr", T0);
    edit(&w);
    let before = snapshot(&w);

    let error = restore_at(&w, &backup, T0 + 60).unwrap_err();
    assert!(manifest_refusal(&error, &backup).contains("belongs to game"));
    assert_eq!(snapshot(&w), before);

    restore_into(&w, "ff2-pr", &backup, &w.source).unwrap();
    assert_eq!(fs::read(w.source.join(SLOT)).unwrap(), b"slot-one");
}

#[test]
fn the_manifest_is_read_from_disk_not_from_the_value() {
    let w = world("restore-the_manifest_is_read_from_disk_not_from_the_value");
    let backup = make(&w, "ff1-pr", T0);
    fs::write(backup.dir.join("backup.json"), manifest_text("ff2-pr", 1, &[SLOT, BESTIARY])).unwrap();
    assert_eq!(backup.manifest.game, "ff1-pr");
    let before = snapshot(&w);
    let error = restore_at(&w, &backup, T0 + 60).unwrap_err();
    assert!(manifest_refusal(&error, &backup).contains("belongs to game"));
    assert_eq!(snapshot(&w), before);

    let mut edited = make(&w, "ff1-pr", T0 + 10);
    edited.manifest.files.push("../escape".into());
    restore_at(&w, &edited, T0 + 60).unwrap();
    assert!(!w.root.join("escape").exists());
    assert!(!edited.dir.join("../escape").exists());
}

#[test]
fn a_missing_backup_file_is_refused_first() {
    let w = world("restore-a_missing_backup_file_is_refused_first");
    let backup = make(&w, "ff1-pr", T0);
    edit(&w);
    fs::remove_file(backup.dir.join(BESTIARY)).unwrap();
    let before = snapshot(&w);

    let error = restore_at(&w, &backup, T0 + 60).unwrap_err();
    let (operation, path, kind) = io_parts(&error);
    assert_eq!(operation, Operation::Read);
    assert_eq!(path, backup.dir.join(BESTIARY));
    assert_eq!(kind, ErrorKind::NotFound);
    assert_eq!(snapshot(&w), before);
}

#[test]
fn a_backup_without_a_manifest_is_refused() {
    let w = world("restore-a_backup_without_a_manifest_is_refused");
    let backup = make(&w, "ff1-pr", T0);
    fs::remove_file(backup.dir.join("backup.json")).unwrap();
    let before = snapshot(&w);

    let error = restore_at(&w, &backup, T0 + 60).unwrap_err();
    let (operation, path, kind) = io_parts(&error);
    assert_eq!(operation, Operation::Read);
    assert_eq!(path, backup.dir.join("backup.json"));
    assert_eq!(kind, ErrorKind::NotFound);
    assert_eq!(snapshot(&w), before);
}

#[test]
fn an_invalid_manifest_is_refused() {
    let w = world("restore-an_invalid_manifest_is_refused");
    let backup = make(&w, "ff1-pr", T0);
    fs::write(w.root.join("outside.txt"), b"sentinel").unwrap();
    let empty_type = manifest_text("ff1-pr", 1, &[SLOT]).replace("\"saveType\": \"slot\"", "\"saveType\": \"\"");
    let cases = [
        manifest_text("ff1-pr", 2, &[SLOT]),
        manifest_text("ff1-pr", 0, &[SLOT]),
        manifest_text("ff1-pr", 1, &[]),
        manifest_text("ff1-pr", 1, &["../x"]),
        manifest_text("ff1-pr", 1, &["a/b"]),
        manifest_text("ff1-pr", 1, &["a\\b"]),
        manifest_text("ff1-pr", 1, &["C:x"]),
        manifest_text("ff1-pr", 1, &["backup.json"]),
        manifest_text("ff1-pr", 1, &[SLOT, SLOT]),
        empty_type,
    ];
    for text in &cases {
        fs::write(backup.dir.join("backup.json"), text).unwrap();
        let before = snapshot(&w);
        let error = restore_at(&w, &backup, T0 + 60).unwrap_err();
        manifest_refusal(&error, &backup);
        assert_eq!(snapshot(&w), before, "{text}");
        assert_eq!(fs::read(w.root.join("outside.txt")).unwrap(), b"sentinel");
        assert!(!w.root.join("x").exists());
    }
}

#[test]
fn a_folder_in_place_of_a_backup_file_is_refused() {
    let w = world("restore-a_folder_in_place_of_a_backup_file_is_refused");
    let backup = make(&w, "ff1-pr", T0);
    fs::remove_file(backup.dir.join(SLOT)).unwrap();
    fs::create_dir(backup.dir.join(SLOT)).unwrap();
    let before = snapshot(&w);

    let error = restore_at(&w, &backup, T0 + 60).unwrap_err();
    let (operation, path, kind) = io_parts(&error);
    assert_eq!(operation, Operation::Read);
    assert_eq!(path, backup.dir.join(SLOT));
    assert_eq!(kind, ErrorKind::InvalidInput);
    assert_eq!(snapshot(&w), before);
}

#[cfg(unix)]
#[test]
fn a_link_in_place_of_a_backup_file_is_refused() {
    let w = world("restore-a_link_in_place_of_a_backup_file_is_refused");
    let backup = make(&w, "ff1-pr", T0);
    fs::write(w.root.join("linked.txt"), b"linked").unwrap();
    fs::remove_file(backup.dir.join(SLOT)).unwrap();
    std::os::unix::fs::symlink(w.root.join("linked.txt"), backup.dir.join(SLOT)).unwrap();
    let before = snapshot(&w);

    let error = restore_at(&w, &backup, T0 + 60).unwrap_err();
    let (operation, path, kind) = io_parts(&error);
    assert_eq!(operation, Operation::Read);
    assert_eq!(path, backup.dir.join(SLOT));
    assert_eq!(kind, ErrorKind::InvalidInput);
    assert_eq!(snapshot(&w), before);
}

#[test]
fn an_invalid_game_id_is_refused() {
    let w = world("restore-an_invalid_game_id_is_refused");
    let backup = make(&w, "ff1-pr", T0);
    let before = snapshot(&w);

    let error = restore_into(&w, "../x", &backup, &w.source).unwrap_err();
    assert!(matches!(error, IoError::InvalidGameId { .. }));
    assert_eq!(snapshot(&w), before);
}

#[test]
fn a_missing_target_folder_is_refused() {
    let w = world("restore-a_missing_target_folder_is_refused");
    let backup = make(&w, "ff1-pr", T0);
    let nowhere = w.root.join("nowhere");
    let before = snapshot(&w);

    let error = restore_into(&w, "ff1-pr", &backup, &nowhere).unwrap_err();
    let (operation, path, kind) = io_parts(&error);
    assert_eq!(operation, Operation::ListDir);
    assert_eq!(path, nowhere);
    assert_eq!(kind, ErrorKind::NotFound);
    assert!(!nowhere.exists());
    assert_eq!(snapshot(&w), before);
}

#[test]
fn a_file_in_place_of_the_target_folder_is_refused() {
    let w = world("restore-a_file_in_place_of_the_target_folder_is_refused");
    let backup = make(&w, "ff1-pr", T0);
    let target = w.root.join("afile");
    fs::write(&target, b"not a folder").unwrap();
    let before = snapshot(&w);

    let error = restore_into(&w, "ff1-pr", &backup, &target).unwrap_err();
    let (operation, path, kind) = io_parts(&error);
    assert_eq!(operation, Operation::ListDir);
    assert_eq!(path, target);
    assert_eq!(kind, ErrorKind::InvalidInput);
    assert_eq!(fs::read(&target).unwrap(), b"not a folder");
    assert_eq!(snapshot(&w), before);
}
