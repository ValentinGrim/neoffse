//! `Registry::open`: read one file and open it with a chosen or any format.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{folder_with, slot_bytes, slot_name};
use neoffse::core::{FormatError, Game};
use neoffse::data::Language;
use neoffse::io::error::{IoError, Operation};
use neoffse::{Error, Registry};

fn registry() -> Registry {
    Registry::new(Language::En)
}

#[test]
fn a_slot_opens_by_name_with_or_without_a_game() {
    let bytes = slot_bytes();
    let dir = folder_with(&[(slot_name(2), &bytes)]);
    for game in [Some(Game::Ff1Pr), None] {
        let save = registry().open(dir.path(), slot_name(2), game).unwrap();
        assert_eq!(save.game, Game::Ff1Pr);
        assert_eq!(save.document.game(), Game::Ff1Pr);
        assert_eq!(save.name, slot_name(2));
        assert_eq!(save.dir, dir.path());
    }
}

#[test]
fn a_renamed_slot_copy_opens_as_ff1_without_a_game() {
    let bytes = slot_bytes();
    let dir = folder_with(&[("renamed-copy.sav", &bytes)]);
    let save = registry().open(dir.path(), "renamed-copy.sav", None).unwrap();
    assert_eq!(save.game, Game::Ff1Pr);
}

#[test]
fn original_holds_the_bytes_read() {
    let bytes = slot_bytes();
    let dir = folder_with(&[(slot_name(2), &bytes)]);
    let save = registry().open(dir.path(), slot_name(2), None).unwrap();
    assert_eq!(save.original.len(), 1);
    assert_eq!(save.original.get(slot_name(2)), Some(bytes.as_slice()));
    // The document encodes back to the bytes it was given.
    let encoded = save.document.encode().unwrap();
    assert_eq!(encoded.get(slot_name(2)), Some(bytes.as_slice()));
}

#[test]
fn a_non_save_file_is_no_format_with_one_attempt_per_format() {
    let dir = folder_with(&[("notes.txt", b"just some text")]);
    let error = registry().open(dir.path(), "notes.txt", None).unwrap_err();
    let Error::NoFormat { attempts } = error else { panic!("expected NoFormat") };
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].0, Game::Ff1Pr);
    assert!(matches!(attempts[0].1, FormatError::Decode { .. }));
}

#[test]
fn a_non_save_file_with_a_game_is_a_format_error() {
    let dir = folder_with(&[("notes.txt", b"just some text")]);
    let error = registry().open(dir.path(), "notes.txt", Some(Game::Ff1Pr)).unwrap_err();
    assert!(matches!(
        error,
        Error::Format {
            source: FormatError::Decode { .. }
        }
    ));
}

#[test]
fn a_game_without_a_format_is_unsupported() {
    let dir = folder_with(&[(slot_name(2), &slot_bytes())]);
    let error = registry().open(dir.path(), slot_name(2), Some(Game::Ff2Pr)).unwrap_err();
    assert!(matches!(error, Error::UnsupportedGame { game: Game::Ff2Pr }));
    assert_eq!(error.to_string(), "no format supports ff2-pr");
}

#[test]
fn a_missing_file_is_an_io_error() {
    let dir = folder_with(&[]);
    let error = registry().open(dir.path(), slot_name(2), None).unwrap_err();
    assert!(matches!(
        error,
        Error::Io {
            source: IoError::Io {
                operation: Operation::Read,
                ..
            }
        }
    ));
}

#[test]
fn a_name_with_a_separator_is_refused_by_the_io_layer() {
    let dir = folder_with(&[(slot_name(2), &slot_bytes())]);
    for name in ["sub/file", "..", "a\\b", ""] {
        let error = registry().open(dir.path(), name, None).unwrap_err();
        assert!(
            matches!(
                error,
                Error::Io {
                    source: IoError::InvalidFileName { .. }
                }
            ),
            "the io layer must answer for {name:?}"
        );
    }
}
