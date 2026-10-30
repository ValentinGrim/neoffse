//! The rows of the Restore page: stamp, reason and slot of each backup.
//! Backups are made with the io crate in temporary folders.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::Path;

use common::{T0, listing, make_backup, rewrite_manifest, slot_bytes, slot_file};
use neoffse::core::Game;
use neoffse::io::dirs::DataDir;
use neoffse_app::restore::{backup_row, backup_rows, reason_of};
use neoffse_app::{BackupReason, SlotKind};
use tempfile::TempDir;

const GAME: &str = "ff1-pr";

/// A data folder and a source folder holding `names`.
fn rig(names: &[&str]) -> (TempDir, TempDir, DataDir) {
    let data_dir = tempfile::tempdir().unwrap();
    let source = tempfile::tempdir().unwrap();
    for name in names {
        fs::write(source.path().join(name), slot_bytes(1)).unwrap();
    }
    let data = DataDir::at(data_dir.path());
    (data_dir, source, data)
}

fn backup_of(data: &DataDir, source: &Path, name: &str, reason: &str, at: i64, offset: i32) -> neoffse::io::backup::Backup {
    make_backup(data, GAME, source, &[name], reason, at, offset)
}

#[test]
fn reason_of_knows_two_reasons() {
    assert_eq!(reason_of("before-save"), BackupReason::BeforeSave);
    assert_eq!(reason_of("before-restore"), BackupReason::BeforeRestore);
    for other in ["pre-upgrade", "", "Before-Save"] {
        assert_eq!(reason_of(other), BackupReason::Other, "reason {other:?}");
    }
}

#[test]
fn a_backup_becomes_a_row() {
    let name = slot_file(3);
    assert_eq!(name.len(), 44);
    let (_keep_data, source, data) = rig(&[name]);
    backup_of(&data, source.path(), name, "before-save", T0, 0);
    let backups = listing(&data, Game::Ff1Pr);
    let row = backup_row(7, &backups[0]);
    assert_eq!(row.key, 7);
    assert_eq!(row.stamp.as_str(), "2020-09-13 12:26 UTC");
    assert_eq!(row.reason, BackupReason::BeforeSave);
    assert_eq!(row.slot_kind, SlotKind::Numbered);
    assert_eq!(row.slot_number, 3);
    assert_eq!(row.file.as_str(), name);
}

#[test]
fn rows_follow_the_listing_order_and_number_the_keys() {
    let name = slot_file(1);
    let (_keep_data, source, data) = rig(&[name]);
    for step in 0..3 {
        backup_of(&data, source.path(), name, "before-save", T0 + 60 * step, 0);
    }
    let rows = backup_rows(&listing(&data, Game::Ff1Pr));
    assert_eq!(rows.iter().map(|row| row.key).collect::<Vec<_>>(), [0, 1, 2]);
    let stamps: Vec<String> = rows.iter().map(|row| row.stamp.to_string()).collect();
    assert_eq!(stamps, ["2020-09-13 12:28 UTC", "2020-09-13 12:27 UTC", "2020-09-13 12:26 UTC"]);
}

#[test]
fn auto_and_quick_files_have_their_kinds() {
    let (auto, quick) = (slot_file(21), slot_file(22));
    let (_keep_data, source, data) = rig(&[auto, quick]);
    backup_of(&data, source.path(), auto, "before-save", T0, 0);
    backup_of(&data, source.path(), quick, "before-save", T0 + 60, 0);
    let rows = backup_rows(&listing(&data, Game::Ff1Pr));
    assert_eq!(rows[0].slot_kind, SlotKind::QuickSave);
    assert_eq!(rows[1].slot_kind, SlotKind::AutoSave);
}

#[test]
fn an_unknown_file_name_shows_the_file() {
    let (_keep_data, source, data) = rig(&["notes.txt"]);
    backup_of(&data, source.path(), "notes.txt", "before-save", T0, 0);
    let row = backup_row(0, &listing(&data, Game::Ff1Pr)[0]);
    assert_eq!(row.slot_kind, SlotKind::Other);
    assert_eq!(row.file.as_str(), "notes.txt");
}

#[test]
fn an_unknown_reason_is_other() {
    let name = slot_file(1);
    let (_keep_data, source, data) = rig(&[name]);
    backup_of(&data, source.path(), name, "pre-upgrade", T0, 0);
    assert_eq!(backup_row(0, &listing(&data, Game::Ff1Pr)[0]).reason, BackupReason::Other);
}

#[test]
fn a_time_that_does_not_parse_gives_an_empty_stamp() {
    let name = slot_file(1);
    let (_keep_data, source, data) = rig(&[name]);
    let backup = backup_of(&data, source.path(), name, "before-save", T0, 0);
    rewrite_manifest(&backup, "created", "yesterday");
    let row = backup_row(0, &listing(&data, Game::Ff1Pr)[0]);
    assert_eq!(row.stamp.as_str(), "");
}

#[test]
fn a_non_utc_time_keeps_its_zone() {
    let name = slot_file(1);
    let (_keep_data, source, data) = rig(&[name]);
    backup_of(&data, source.path(), name, "before-save", T0, 7200);
    let row = backup_row(0, &listing(&data, Game::Ff1Pr)[0]);
    assert_eq!(row.stamp.as_str(), "2020-09-13 14:26 +02:00");
}

#[test]
fn no_row_string_holds_the_source_folder() {
    let name = slot_file(1);
    let (_keep_data, source, data) = rig(&[name]);
    backup_of(&data, source.path(), name, "before-save", T0, 0);
    let folder = source.path().to_string_lossy().into_owned();
    assert!(folder.len() > 8, "the folder text must be distinctive");
    for row in backup_rows(&listing(&data, Game::Ff1Pr)) {
        for text in [row.stamp.as_str(), row.file.as_str()] {
            assert!(!text.contains(&folder), "a row string holds the source folder");
        }
    }
}
