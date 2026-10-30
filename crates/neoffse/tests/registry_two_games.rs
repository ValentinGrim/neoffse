//! A registry that holds FF I and a second real `SlotFormat`: the game rule
//! of the shared slot pipeline decides which format opens a file, and its
//! `OpenError`s tell the facade which game owns a refused one.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::path::Path;

use common::{ff2_registry, folder_with, must, slot_bytes, slot_name, slot_with_text_replaced, slot_with_vehicles};
use neoffse::core::{FormatError, Game};
use neoffse::pr::OpenError;
use neoffse::{Error, Registry};

const NAME: &str = "slot.bin";

/// The error of opening `bytes` as `NAME` in `registry`.
#[track_caller]
fn failure(registry: &Registry, bytes: &[u8], game: Option<Game>) -> Error {
    let dir = folder_with(&[(NAME, bytes)]);
    failure_in(registry, dir.path(), NAME, game)
}

#[track_caller]
fn failure_in(registry: &Registry, dir: &Path, name: &str, game: Option<Game>) -> Error {
    let Err(error) = registry.open(dir, name, game) else {
        panic!("the open must fail");
    };
    error
}

/// The game of the session that opens `bytes` as `NAME`. The summary is not
/// read: the FF II stand-in summarizes with FF I code.
#[track_caller]
fn opened(registry: &Registry, bytes: &[u8], game: Option<Game>) -> Game {
    let dir = folder_with(&[(NAME, bytes)]);
    must(registry.open(dir.path(), NAME, game), "open the slot").game()
}

/// The `OpenError` that a refusal of a format carries.
#[track_caller]
fn open_error(error: &FormatError) -> &OpenError {
    let FormatError::Decode { source } = error else {
        panic!("expected a Decode error");
    };
    source.downcast_ref::<OpenError>().expect("the source is an OpenError")
}

/// The `(game, found, vehicles)` of each attempt of a `NoFormat`.
#[track_caller]
fn attempts_of(error: &Error) -> Vec<(Game, Option<Game>, usize)> {
    let Error::NoFormat { attempts } = error else {
        panic!("expected NoFormat");
    };
    attempts
        .iter()
        .map(|(game, error)| match open_error(error) {
            OpenError::OtherGame { found, vehicles } => (*game, *found, *vehicles),
            other => panic!("expected OtherGame, got {other:?}"),
        })
        .collect()
}

/// The `OtherGame` of a refusal by the format of a named game.
#[track_caller]
fn refused_as(error: &Error) -> (Option<Game>, usize) {
    let Error::Format { source } = error else {
        panic!("expected Error::Format");
    };
    match open_error(source) {
        OpenError::OtherGame { found, vehicles } => (*found, *vehicles),
        other => panic!("expected OtherGame, got {other:?}"),
    }
}

#[test]
fn an_ff2_slot_opens_as_ff2() {
    let registry = ff2_registry();
    let bytes = slot_with_vehicles(7);
    for game in [None, Some(Game::Ff2Pr)] {
        assert_eq!(opened(&registry, &bytes, game), Game::Ff2Pr, "game {game:?}");
    }
}

#[test]
fn an_ff1_slot_still_opens_as_ff1() {
    let registry = ff2_registry();
    let bytes = slot_bytes();
    for game in [None, Some(Game::Ff1Pr)] {
        assert_eq!(opened(&registry, &bytes, game), Game::Ff1Pr, "game {game:?}");
    }
}

#[test]
fn a_slot_of_one_game_never_opens_as_the_other() {
    let registry = ff2_registry();
    let error = failure(&registry, &slot_with_vehicles(7), Some(Game::Ff1Pr));
    assert_eq!(refused_as(&error), (Some(Game::Ff2Pr), 7));
    assert!(!registry.is_unsupported_game(&error));

    let error = failure(&registry, &slot_bytes(), Some(Game::Ff2Pr));
    assert_eq!(refused_as(&error), (Some(Game::Ff1Pr), 4));
    assert!(!registry.is_unsupported_game(&error));
}

#[test]
fn a_slot_of_an_unregistered_game_is_unsupported() {
    let registry = ff2_registry();
    let error = failure(&registry, &slot_with_vehicles(14), None);
    let attempts = attempts_of(&error);
    assert_eq!(attempts, vec![(Game::Ff1Pr, Some(Game::Ff3Pr), 14), (Game::Ff2Pr, Some(Game::Ff3Pr), 14)]);
    assert!(registry.is_unsupported_game(&error));
}

#[test]
fn a_count_no_game_has_is_not_unsupported() {
    let registry = ff2_registry();
    let error = failure(&registry, &slot_with_vehicles(5), None);
    let attempts = attempts_of(&error);
    assert_eq!(attempts, vec![(Game::Ff1Pr, None, 5), (Game::Ff2Pr, None, 5)]);
    assert!(!registry.is_unsupported_game(&error));
}

#[test]
fn a_broken_slot_of_a_registered_game_is_not_unsupported() {
    let registry = ff2_registry();
    // The key sits inside the embedded document, so the replace works on the raw text.
    let bytes = slot_with_text_replaced(7, "importantOwendItemList", "importantOwendItemListX");
    let error = failure(&registry, &bytes, None);
    let Error::NoFormat { attempts } = &error else {
        panic!("expected NoFormat");
    };
    assert_eq!(attempts.len(), 2);
    let games: Vec<Game> = attempts.iter().map(|(game, _)| *game).collect();
    assert_eq!(games, vec![Game::Ff1Pr, Game::Ff2Pr]);
    assert!(matches!(
        open_error(&attempts[0].1),
        OpenError::OtherGame {
            found: Some(Game::Ff2Pr),
            vehicles: 7
        }
    ));
    assert!(matches!(open_error(&attempts[1].1), OpenError::NotASlot { .. }));
    assert!(!registry.is_unsupported_game(&error));
}

#[test]
fn a_scan_without_a_folder_game_claims_by_name_and_open_refuses_a_wrong_claim() {
    let registry = ff2_registry();
    let (ff2_name, ff1_name) = (slot_name(2), slot_name(3));
    let dir = folder_with(&[(ff2_name, &slot_with_vehicles(7)), (ff1_name, &slot_bytes())]);

    // Equal claims go to the first game of `Game::ALL`, owner or not.
    let scan = must(registry.scan(dir.path(), None), "scan without a folder game");
    assert_eq!(scan.saves.len(), 2);
    assert!(scan.saves.iter().all(|found| found.claim.game == Game::Ff1Pr));
    let found = scan.saves.iter().find(|found| found.name == ff2_name).expect("the FF II file is claimed");
    let error = failure_in(&registry, dir.path(), ff2_name, Some(found.claim.game));
    assert_eq!(refused_as(&error), (Some(Game::Ff2Pr), 7));

    // Naming FF II claims both files again, and the FF I file is refused.
    let scan = must(registry.scan(dir.path(), Some(Game::Ff2Pr)), "scan in the FF II folder");
    assert_eq!(scan.saves.len(), 2);
    assert!(scan.saves.iter().all(|found| found.claim.game == Game::Ff2Pr));
    let error = failure_in(&registry, dir.path(), ff1_name, Some(Game::Ff2Pr));
    assert_eq!(refused_as(&error), (Some(Game::Ff1Pr), 4));
}
