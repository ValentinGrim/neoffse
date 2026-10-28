//! How a finished restore pipeline maps to the outcome the page shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::Error;
use neoffse::io::error::IoError;
use neoffse_app::restore::{RestoreOutcome, outcome_of};

fn restore_failed(written: &[&str], remaining: &[&str]) -> Error {
    Error::Io {
        source: IoError::RestoreFailed {
            written: written.iter().map(|name| (*name).to_owned()).collect(),
            remaining: remaining.iter().map(|name| (*name).to_owned()).collect(),
            safety: None,
            source: Box::new(IoError::NoDataDir),
        },
    }
}

#[test]
fn a_first_write_failure_is_failed() {
    assert_eq!(outcome_of(Err(restore_failed(&[], &["a"])), false), RestoreOutcome::Failed);
}

#[test]
fn a_later_write_failure_is_incomplete() {
    assert_eq!(outcome_of(Err(restore_failed(&["a"], &["b"])), false), RestoreOutcome::Incomplete);
}

#[test]
fn other_errors_keep_their_outcome() {
    let running = Error::GameRunning { executables: Vec::new() };
    assert_eq!(outcome_of(Err(running), false), RestoreOutcome::GameRunning);
    let io = Error::Io { source: IoError::NoDataDir };
    assert_eq!(outcome_of(Err(io), false), RestoreOutcome::Failed);
}
