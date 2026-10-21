//! `IoError` and `Operation`: messages, sources and the non-exhaustive
//! contract.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::error::Error as _;
use std::path::PathBuf;

use neoffse_io::error::{IoError, Operation};

const MARKER: &str = "marker-7f3a";
const PATH: &str = "some/dir/save.sav";

fn boxed() -> Box<dyn std::error::Error + Send + Sync> {
    Box::<dyn std::error::Error + Send + Sync>::from(MARKER)
}

fn io_error(operation: Operation) -> IoError {
    IoError::Io {
        operation,
        path: PathBuf::from(PATH),
        source: std::io::Error::other(MARKER),
    }
}

fn every_error() -> Vec<IoError> {
    vec![
        io_error(Operation::Read),
        IoError::NoDataDir,
        IoError::InvalidGameId { game: "Bad/Id".to_owned() },
        IoError::VerificationFailed {
            path: PathBuf::from(PATH),
            source: boxed(),
        },
        IoError::ManifestInvalid {
            path: PathBuf::from(PATH),
            source: boxed(),
        },
        IoError::LibraryListInvalid {
            path: PathBuf::from(PATH),
            source: boxed(),
        },
        IoError::InvalidFileName { name: "a/b".to_owned() },
        IoError::FileTooLarge {
            path: PathBuf::from(PATH),
            limit: 10,
        },
    ]
}

fn every_operation() -> [Operation; 9] {
    [
        Operation::CreateDir,
        Operation::CreateTemp,
        Operation::Read,
        Operation::Write,
        Operation::Sync,
        Operation::Copy,
        Operation::Rename,
        Operation::Remove,
        Operation::ListDir,
    ]
}

#[test]
fn io_error_names_the_operation_and_the_path() {
    let path = PathBuf::from(PATH);
    for operation in every_operation() {
        let message = io_error(operation).to_string();
        assert!(message.contains(&operation.to_string()), "{message}");
        assert!(message.contains(&path.display().to_string()), "{message}");
    }
}

#[test]
fn operation_texts_are_all_different() {
    let texts: Vec<String> = every_operation().iter().map(ToString::to_string).collect();
    for (i, text) in texts.iter().enumerate() {
        assert_ne!(text, "");
        for other in &texts[i + 1..] {
            assert_ne!(text, other);
        }
    }
}

#[test]
fn display_never_repeats_the_source() {
    let with_cause = [
        io_error(Operation::Write),
        IoError::VerificationFailed {
            path: PathBuf::from(PATH),
            source: boxed(),
        },
        IoError::ManifestInvalid {
            path: PathBuf::from(PATH),
            source: boxed(),
        },
        IoError::LibraryListInvalid {
            path: PathBuf::from(PATH),
            source: boxed(),
        },
    ];
    for error in &with_cause {
        assert!(!error.to_string().contains(MARKER), "{error}");
        let source = error.source().unwrap();
        assert_eq!(source.to_string(), MARKER);
    }
}

#[test]
fn variants_without_a_cause_have_no_source() {
    assert!(IoError::NoDataDir.source().is_none());
    assert!(IoError::InvalidGameId { game: String::new() }.source().is_none());
    assert!(IoError::InvalidFileName { name: String::new() }.source().is_none());
    let too_large = IoError::FileTooLarge {
        path: PathBuf::from(PATH),
        limit: 10,
    };
    assert!(too_large.source().is_none());
}

#[test]
fn file_name_and_size_errors_show_their_subject() {
    let name = IoError::InvalidFileName { name: "a/b".to_owned() };
    assert_eq!(name.to_string(), "invalid file name \"a/b\"");
    let too_large = IoError::FileTooLarge {
        path: PathBuf::from(PATH),
        limit: 10,
    };
    let message = too_large.to_string();
    assert!(message.contains(PATH), "{message}");
    assert!(message.contains("10"), "{message}");
}

#[test]
fn every_path_variant_shows_its_path() {
    let shown = PathBuf::from(PATH).display().to_string();
    let with_path = [
        io_error(Operation::Read),
        IoError::VerificationFailed {
            path: PathBuf::from(PATH),
            source: boxed(),
        },
        IoError::ManifestInvalid {
            path: PathBuf::from(PATH),
            source: boxed(),
        },
        IoError::LibraryListInvalid {
            path: PathBuf::from(PATH),
            source: boxed(),
        },
    ];
    for error in &with_path {
        assert!(error.to_string().contains(&shown), "{error}");
    }
}

#[test]
fn invalid_game_id_shows_the_id() {
    let error = IoError::InvalidGameId { game: "Bad/Id".to_owned() };
    assert!(error.to_string().contains("Bad/Id"));
}

#[test]
fn match_needs_a_wildcard_arm() {
    let mut seen = Vec::new();
    for error in every_error() {
        seen.push(match error {
            IoError::Io { .. } => 1,
            IoError::NoDataDir => 2,
            IoError::InvalidGameId { .. } => 3,
            IoError::VerificationFailed { .. } => 4,
            IoError::ManifestInvalid { .. } => 5,
            IoError::LibraryListInvalid { .. } => 6,
            IoError::InvalidFileName { .. } => 7,
            IoError::FileTooLarge { .. } => 8,
            _ => 0,
        });
    }
    assert_eq!(seen, [1, 2, 3, 4, 5, 6, 7, 8]);

    let mut seen = Vec::new();
    for operation in every_operation() {
        seen.push(match operation {
            Operation::CreateDir => 1,
            Operation::CreateTemp => 2,
            Operation::Read => 3,
            Operation::Write => 4,
            Operation::Sync => 5,
            Operation::Copy => 6,
            Operation::Rename => 7,
            Operation::Remove => 8,
            Operation::ListDir => 9,
            _ => 0,
        });
    }
    assert_eq!(seen, [1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn errors_are_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<IoError>();
    assert_send_sync::<Operation>();
}
