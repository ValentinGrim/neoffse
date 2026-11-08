//! The shipped registry against the owner's FF II to VI saves
//! (`NEOFFSE_SAVES`, read in place and never written). The tests skip a
//! folder the saves lack, and print counts, never file names or values.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{folder_of, is_slot_name, private_files_of};
use neoffse::core::Game;
use neoffse::data::Language;
use neoffse::{Error, Registry};
use neoffse_testkit::report;

/// The game folders `ff3` to `ff6`, which the shipped registry has no format
/// for.
const UNSUPPORTED: [&str; 4] = ["ff3", "ff4", "ff5", "ff6"];

#[test]
fn every_private_ff2_slot_opens_as_ff2() {
    let Some(files) = private_files_of("ff2") else { return };
    let registry = Registry::new(Language::En);
    let mut opened = 0;
    for (index, (name, path)) in files.iter().enumerate().filter(|(_, (name, _))| is_slot_name(name)) {
        for game in [None, Some(Game::Ff2Pr)] {
            let Ok(session) = registry.open(folder_of(path), name, game) else {
                panic!("ff2 slot {index} must open with game {game:?}");
            };
            assert_eq!(session.game(), Game::Ff2Pr, "ff2 slot {index} must open as FF II");
        }
        opened += 1;
    }
    report(&format!("ff2: {opened} slot files opened as FF II, {} files listed", files.len()));
}

#[test]
fn a_private_ff2_file_that_is_no_slot_never_opens_as_ff2() {
    let Some(files) = private_files_of("ff2") else { return };
    let registry = Registry::new(Language::En);
    for (index, (name, path)) in files.iter().enumerate().filter(|(_, (name, _))| !is_slot_name(name)) {
        let result = registry.open(folder_of(path), name, None);
        assert!(result.is_err(), "ff2 file {index} is no slot and must not open");
    }
}

#[test]
fn every_private_slot_of_ff3_to_ff6_is_unsupported() {
    let registry = Registry::new(Language::En);
    for folder in UNSUPPORTED {
        let Some(files) = private_files_of(folder) else { continue };
        let mut refused = 0;
        for (index, (name, path)) in files.iter().enumerate().filter(|(_, (name, _))| is_slot_name(name)) {
            let Err(error) = registry.open(folder_of(path), name, None) else {
                panic!("{folder} slot {index} must not open");
            };
            assert!(
                matches!(error, Error::NoFormat { .. }) && registry.is_unsupported_game(&error),
                "{folder} slot {index} must read as an unsupported game"
            );
            refused += 1;
        }
        report(&format!("{folder}: {refused} slot files refused as an unsupported game"));
    }
}
