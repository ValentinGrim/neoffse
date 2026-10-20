//! `SaveDocument::verify`, `open`, `SaveFormat::detect` and `FormatError`,
//! driven through trait objects over the fake format.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::error::Error;

use common::fake::{FILE, FakeDocument, FakeFormat, HEADER, add, bytes, doc, files_of};
use neoffse_core::{CapPolicy, Confidence, FileProbe, FormatError, Game, SaveDocument, SaveFiles, SaveFormat, SaveSlot};

#[test]
fn verify_accepts_what_encode_wrote() {
    let mut document = doc();
    document.apply(&add(5, None, 4), CapPolicy::Enforce).unwrap();
    document.verify(&document.encode().unwrap()).unwrap();
}

#[test]
fn verify_rejects_a_changed_value_byte() {
    let document = doc();
    let text = String::from_utf8(bytes(&*document)).unwrap();
    let altered = text.replace("gil=i:100", "gil=i:101");
    assert_ne!(altered, text);
    let error = document.verify(&files_of(altered.into_bytes())).unwrap_err();
    assert!(matches!(error, FormatError::VerifyMismatch { .. } | FormatError::Decode { .. }));
}

#[test]
fn verify_sees_what_the_view_omits() {
    let mut document = FakeDocument::sample();
    let files = document.encode().unwrap();
    let view = document.view().unwrap();
    document.set_hidden("kept apart");
    assert_eq!(document.view().unwrap(), view);
    let error = document.verify(&files).unwrap_err();
    assert!(matches!(error, FormatError::VerifyMismatch { name } if name == FILE));
}

#[test]
fn verify_sees_a_changed_row_order() {
    let document = doc();
    let text = String::from_utf8(bytes(&*document)).unwrap();
    let mut lines: Vec<&str> = text.lines().collect();
    let last = lines.len() - 1;
    lines.swap(last - 1, last);
    let swapped = format!("{}\n", lines.join("\n"));
    assert_ne!(swapped, text);
    let error = document.verify(&files_of(swapped.into_bytes())).unwrap_err();
    assert!(matches!(error, FormatError::VerifyMismatch { ref name } if name == FILE), "{error:?}");
}

#[test]
fn verify_reports_missing_and_unreadable_files() {
    let document = doc();
    let error = document.verify(&SaveFiles::new()).unwrap_err();
    assert!(matches!(error, FormatError::MissingFile { name } if name == FILE));
    let error = document.verify(&files_of(b"garbage".to_vec())).unwrap_err();
    assert!(matches!(&error, FormatError::Decode { .. }));
    assert!(error.source().is_some());
}

#[test]
fn verify_changes_nothing() {
    let document = doc();
    let before = bytes(&*document);
    document.verify(&document.encode().unwrap()).unwrap();
    document.verify(&files_of(b"garbage".to_vec())).unwrap_err();
    assert_eq!(bytes(&*document), before);
}

#[test]
fn open_fails_on_unreadable_or_missing_files() {
    let format: &dyn SaveFormat = &FakeFormat;
    let error = format.open(&files_of(b"garbage".to_vec())).err().unwrap();
    assert!(matches!(error, FormatError::Decode { .. }));
    let error = format.open(&SaveFiles::new()).err().unwrap();
    assert!(matches!(error, FormatError::MissingFile { name } if name == FILE));
}

#[test]
fn detect_reads_the_name_and_the_first_bytes() {
    let format: &dyn SaveFormat = &FakeFormat;
    let certain = format.detect(&FileProbe::new(FILE, 100, HEADER.as_bytes())).unwrap();
    assert_eq!(certain.confidence, Confidence::Certain);
    assert_eq!(certain.slot, Some(SaveSlot::Numbered(1)));
    let likely = format.detect(&FileProbe::new(FILE, 100, b"other")).unwrap();
    assert_eq!(likely.confidence, Confidence::Likely);
    assert!(format.detect(&FileProbe::new("other.sav", 100, HEADER.as_bytes())).is_none());
    assert!(format.detect(&FileProbe::new(".neoffse-abc123.tmp", 100, HEADER.as_bytes())).is_none());
    assert_eq!(format.game(), Game::Ff1Pr);
}

#[test]
fn format_errors_display_without_repeating_the_source() {
    let cause = || -> Box<dyn Error + Send + Sync> { Box::new(std::io::Error::other("cause-7d2a")) };
    for error in [FormatError::Decode { source: cause() }, FormatError::Encode { source: cause() }] {
        assert!(!error.to_string().contains("cause-7d2a"));
        let source = error.source().unwrap().downcast_ref::<std::io::Error>().unwrap();
        assert_eq!(source.to_string(), "cause-7d2a");
    }
    let name = "slot-name.sav".to_string();
    for error in [
        FormatError::MissingFile { name: name.clone() },
        FormatError::DuplicateFile { name: name.clone() },
        FormatError::InvalidFileName { name: name.clone() },
        FormatError::VerifyMismatch { name: name.clone() },
    ] {
        assert!(error.to_string().contains(&name), "{error}");
    }
}
