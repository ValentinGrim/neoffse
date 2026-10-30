//! An FF I registry against the owner's FF II to VI saves (`NEOFFSE_SAVES`,
//! read in place and never written). The tests skip a folder the saves lack,
//! and print counts, never file names or values.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::path::Path;

use common::{ff1_registry, must_some, private_files_of};
use neoffse::Error;
use neoffse::core::{FormatError, Game};
use neoffse::pr::OpenError;
use neoffse::pr::slots::SLOT_FILES;

/// The game folders `ff2` to `ff6` with the game of each and the vehicle
/// count its slots hold (`docs/formats/pixel-remaster.md#telling-the-games-apart`).
const FOLDERS: [(&str, Game, usize); 5] = [
    ("ff2", Game::Ff2Pr, 7),
    ("ff3", Game::Ff3Pr, 14),
    ("ff4", Game::Ff4Pr, 11),
    ("ff5", Game::Ff5Pr, 19),
    ("ff6", Game::Ff6Pr, 8),
];

#[allow(clippy::print_stderr, reason = "the counts of the run are the only output a skipped-or-not test has")]
fn report(line: &str) {
    eprintln!("{line}");
}

fn is_slot_name(name: &str) -> bool {
    SLOT_FILES.iter().any(|file| file.name == name)
}

/// The folder of a private file.
fn folder_of(path: &Path) -> &Path {
    must_some(path.parent(), "a private file has a folder")
}

#[test]
fn every_private_slot_of_ff2_to_ff6_is_unsupported_for_an_ff1_registry() {
    let registry = ff1_registry();
    for (position, (folder, game, vehicles)) in FOLDERS.into_iter().enumerate() {
        assert_eq!(Game::ALL.get(position + 1), Some(&game), "{folder} is the game after {position} others");
        let Some(files) = private_files_of(folder) else { continue };
        let mut opened = 0;
        for (index, (name, path)) in files.iter().enumerate().filter(|(_, (name, _))| is_slot_name(name)) {
            let dir = folder_of(path);
            let Err(error) = registry.open(dir, name, None) else {
                panic!("{folder} slot {index} must not open as FF I");
            };
            assert!(registry.is_unsupported_game(&error), "{folder} slot {index} must read as an unsupported game");

            let Err(Error::Format {
                source: FormatError::Decode { source },
            }) = registry.open(dir, name, Some(Game::Ff1Pr))
            else {
                panic!("{folder} slot {index}: the FF I folder must refuse it with a Decode error");
            };
            let found = source.downcast_ref::<OpenError>();
            assert!(
                matches!(found, Some(OpenError::OtherGame { found: Some(owner), vehicles: count }) if *owner == game && *count == vehicles),
                "{folder} slot {index} must name {} with {vehicles} vehicles",
                game.id()
            );
            opened += 1;
        }
        report(&format!(
            "{folder}: {opened} slot files refused as the game they belong to, {} files listed",
            files.len()
        ));
    }
}

#[test]
fn a_private_file_that_is_no_slot_is_never_unsupported() {
    let registry = ff1_registry();
    for (folder, ..) in FOLDERS {
        let Some(files) = private_files_of(folder) else { continue };
        let mut checked = 0;
        for (index, (name, path)) in files.iter().enumerate().filter(|(_, (name, _))| !is_slot_name(name)) {
            let Err(error) = registry.open(folder_of(path), name, None) else {
                panic!("{folder} file {index} is no slot and must not open");
            };
            assert!(
                !registry.is_unsupported_game(&error),
                "{folder} file {index} is no slot and must not read as unsupported"
            );
            checked += 1;
        }
        report(&format!("{folder}: {checked} non-slot files refused, none as an unsupported game"));
    }
}
