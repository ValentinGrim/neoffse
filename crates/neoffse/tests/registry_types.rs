//! Thread bounds, `Debug` and the error chain of the facade types.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::error::Error as _;

use common::{folder_with, slot_bytes, slot_name};
use neoffse::core::{FormatError, Game};
use neoffse::data::Language;
use neoffse::io::error::IoError;
use neoffse::{Error, Registry, Scan, Session};

fn is_send<T: Send>() {}
fn is_send_sync<T: Send + Sync>() {}

#[test]
fn the_types_cross_threads() {
    is_send_sync::<Registry>();
    is_send_sync::<Error>();
    is_send_sync::<Scan>();
    is_send::<Session>();
}

#[test]
fn debug_of_a_session_holds_the_game_and_the_name_only() {
    let bytes = slot_bytes();
    let dir = folder_with(&[(slot_name(2), &bytes)]);
    let save = Registry::new(Language::En).open(dir.path(), slot_name(2), None).unwrap();
    let text = format!("{save:?}");
    assert!(text.contains("ff1-pr"));
    assert!(text.contains(slot_name(2)));
    // No byte of the file, and no folder path.
    let body = String::from_utf8_lossy(&bytes[3..40]).into_owned();
    assert!(!text.contains(&body));
    assert!(!text.contains(&dir.path().display().to_string()));
}

#[test]
fn io_and_format_errors_expose_their_source() {
    let io = Error::Io { source: IoError::NoDataDir };
    assert_eq!(io.to_string(), "cannot access the save files");
    assert!(io.source().is_some());

    let format = Error::Format {
        source: FormatError::InvalidFileName { name: String::new() },
    };
    assert_eq!(format.to_string(), "the save files do not open");
    assert!(format.source().is_some());
}

#[test]
fn unsupported_game_and_no_format_have_no_source() {
    let unsupported = Error::UnsupportedGame { game: Game::Ff4Pr };
    assert_eq!(unsupported.to_string(), "no format supports ff4-pr");
    assert!(unsupported.source().is_none());

    let none = Error::NoFormat { attempts: Vec::new() };
    assert_eq!(none.to_string(), "no format opens the file");
    assert!(none.source().is_none());
}
