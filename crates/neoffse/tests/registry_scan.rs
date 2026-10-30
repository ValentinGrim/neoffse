//! `Registry::scan`: which files of a folder a format claims.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{folder_with, slot_bytes, slot_name, write_in};
use neoffse::core::{Game, SaveSlot};
use neoffse::data::Language;
use neoffse::io::error::IoError;
use neoffse::{Error, Registry};

fn registry() -> Registry {
    Registry::new(Language::En)
}

#[test]
fn a_folder_lists_its_slot_files_only_in_name_order() {
    let bytes = slot_bytes();
    let (first, second) = (slot_name(2), slot_name(3));
    let dir = folder_with(&[
        (second, &bytes),
        (first, &bytes),
        (".neoffse-x.tmp", &bytes),
        ("steam_autocloud.vdf", b"\"AutoCloud\""),
        ("renamed-copy.sav", &bytes),
    ]);
    fs::create_dir(dir.path().join("subfolder")).unwrap();

    for folder_game in [Some(Game::Ff1Pr), None] {
        let scan = registry().scan(dir.path(), folder_game).unwrap();
        let mut expected = vec![first, second];
        expected.sort_unstable();
        let names: Vec<&str> = scan.saves.iter().map(|found| found.name.as_str()).collect();
        assert_eq!(names, expected);
        assert!(scan.saves.iter().all(|found| found.claim.game == Game::Ff1Pr));
        assert_eq!(scan.non_utf8, 0);
        assert!(scan.unreadable.is_empty());
    }
}

#[test]
fn a_claim_carries_the_slot_of_the_name() {
    let dir = folder_with(&[(slot_name(1), &slot_bytes())]);
    let scan = registry().scan(dir.path(), Some(Game::Ff1Pr)).unwrap();
    assert_eq!(scan.saves.len(), 1);
    assert_eq!(scan.saves[0].claim.detection.slot, Some(SaveSlot::Quick));
}

#[test]
fn another_folder_game_finds_nothing() {
    let dir = folder_with(&[(slot_name(2), &slot_bytes())]);
    let scan = registry().scan(dir.path(), Some(Game::Ff3Pr)).unwrap();
    assert_eq!(scan.saves, vec![]);
}

#[test]
fn an_empty_folder_scans_to_nothing() {
    let dir = folder_with(&[]);
    let scan = registry().scan(dir.path(), None).unwrap();
    assert_eq!(scan.saves, vec![]);
}

#[test]
fn a_missing_folder_is_an_io_error() {
    let dir = folder_with(&[]);
    let missing = dir.path().join("absent");
    let error = registry().scan(&missing, None).unwrap_err();
    assert!(matches!(error, Error::Io { source: IoError::Io { .. } }));
}

#[test]
fn a_file_added_after_the_first_scan_shows_in_the_next() {
    let dir = folder_with(&[]);
    let registry = registry();
    assert_eq!(registry.scan(dir.path(), None).unwrap().saves, vec![]);
    write_in(dir.path(), slot_name(4), &slot_bytes());
    assert_eq!(registry.scan(dir.path(), None).unwrap().saves.len(), 1);
}
