//! `Registry::is_unsupported_game`: which errors say "a game this registry has
//! no format for owns the file".
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::path::Path;

use common::{Refusal, StandIn, ff1_registry, folder_with};
use neoffse::core::{FormatError, Game};
use neoffse::{Error, Registry};

const NAME: &str = "any.bin";

/// A registry of stand-ins that each refuse every file with their refusal.
fn refusing(refusals: &[(Game, Refusal)]) -> Registry {
    Registry::from_formats(refusals.iter().map(|(game, refusal)| StandIn::new(*game).refusing(*refusal).arc()).collect())
}

/// The error of opening `name` of `dir` in `registry`.
#[track_caller]
fn failure_in(registry: &Registry, dir: &Path, name: &str, game: Option<Game>) -> Error {
    let Err(error) = registry.open(dir, name, game) else {
        panic!("the open must fail");
    };
    error
}

/// The error of opening `any.bin` (the bytes `x`) in `registry`.
#[track_caller]
fn failure(registry: &Registry, game: Option<Game>) -> Error {
    let dir = folder_with(&[(NAME, b"x")]);
    failure_in(registry, dir.path(), NAME, game)
}

fn other_game(game: Game) -> Refusal {
    Refusal::OtherGame(Some(game))
}

#[test]
fn an_unregistered_game_is_unsupported() {
    let registry = ff1_registry();
    let error = failure(&registry, Some(Game::Ff4Pr));
    assert!(matches!(error, Error::UnsupportedGame { game: Game::Ff4Pr }));
    assert!(registry.is_unsupported_game(&error));
}

#[test]
fn every_attempt_naming_an_unregistered_game_is_unsupported() {
    let registry = refusing(&[(Game::Ff1Pr, other_game(Game::Ff2Pr))]);
    let error = failure(&registry, None);
    assert!(matches!(&error, Error::NoFormat { attempts } if attempts.len() == 1));
    assert!(registry.is_unsupported_game(&error));

    let registry = refusing(&[(Game::Ff1Pr, other_game(Game::Ff2Pr)), (Game::Ff3Pr, other_game(Game::Ff2Pr))]);
    let error = failure(&registry, None);
    assert!(matches!(&error, Error::NoFormat { attempts } if attempts.len() == 2));
    assert!(registry.is_unsupported_game(&error));
}

#[test]
fn an_attempt_naming_a_registered_game_is_not_unsupported() {
    // The owner of the file has a format and refused it as a broken slot.
    let registry = refusing(&[(Game::Ff1Pr, other_game(Game::Ff2Pr)), (Game::Ff2Pr, Refusal::Garbage)]);
    assert!(!registry.is_unsupported_game(&failure(&registry, None)));

    // Two registered games that each name the other.
    let registry = refusing(&[(Game::Ff1Pr, other_game(Game::Ff6Pr)), (Game::Ff6Pr, other_game(Game::Ff1Pr))]);
    assert!(!registry.is_unsupported_game(&failure(&registry, None)));

    // The same refusal alone: the named game has no format.
    let registry = refusing(&[(Game::Ff1Pr, other_game(Game::Ff6Pr))]);
    assert!(registry.is_unsupported_game(&failure(&registry, None)));
}

#[test]
fn one_attempt_that_names_no_game_is_not_unsupported() {
    for second in [Refusal::Garbage, Refusal::MissingFile, Refusal::OtherGame(None)] {
        let registry = refusing(&[(Game::Ff1Pr, other_game(Game::Ff4Pr)), (Game::Ff2Pr, second)]);
        assert!(!registry.is_unsupported_game(&failure(&registry, None)));
    }
}

#[test]
fn an_empty_attempt_list_is_not_unsupported() {
    let registry = Registry::from_formats(Vec::new());
    let error = failure(&registry, None);
    assert!(matches!(&error, Error::NoFormat { attempts } if attempts.is_empty()));
    assert!(!registry.is_unsupported_game(&error));
}

#[test]
fn a_refusal_from_a_game_folder_is_not_unsupported() {
    let registry = refusing(&[(Game::Ff1Pr, other_game(Game::Ff2Pr))]);
    let error = failure(&registry, Some(Game::Ff1Pr));
    assert!(matches!(
        &error,
        Error::Format {
            source: FormatError::Decode { .. }
        }
    ));
    assert!(!registry.is_unsupported_game(&error));
}

#[test]
fn the_other_errors_are_false() {
    let registry = ff1_registry();
    let dir = folder_with(&[]);
    let io = failure_in(&registry, dir.path(), NAME, None);
    assert!(matches!(io, Error::Io { .. }));
    assert!(!registry.is_unsupported_game(&io));
    assert!(!registry.is_unsupported_game(&Error::GameRunning { executables: vec![] }));
    assert!(!registry.is_unsupported_game(&Error::ChangedOnDisk { name: String::new() }));
}

#[test]
fn a_real_garbage_file_is_not_unsupported() {
    let registry = ff1_registry();
    let dir = folder_with(&[("notes.txt", b"just some text")]);
    let error = failure_in(&registry, dir.path(), "notes.txt", None);
    assert!(matches!(&error, Error::NoFormat { .. }));
    assert!(!registry.is_unsupported_game(&error));
}

#[test]
fn open_error_has_one_type_at_both_paths() {
    let _: fn(neoffse::pr::OpenError) -> neoffse::pr::ff1::OpenError = |error| error;
}
