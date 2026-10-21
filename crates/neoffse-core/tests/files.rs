//! `SaveFiles`, `FileProbe`, `Confidence` and `Detection`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_core::{Confidence, Detection, FileProbe, FormatError, PROBE_HEAD_LEN, SaveFiles, SaveSlot};

fn three() -> SaveFiles {
    let mut files = SaveFiles::new();
    files.push("b", vec![1]).unwrap();
    files.push("a", vec![2, 2]).unwrap();
    files.push("c", vec![]).unwrap();
    files
}

#[test]
fn push_accepts_the_slot_names_of_the_format_document() {
    let mut files = SaveFiles::new();
    for name in [
        "7nCxyzTwG31W3Zlg70mo751W8ETH1n+Km0dWOzRU84Y=",
        "Rl18osV3e9kPX9SMWQj8mqShFpTUmu1lf6Mb=FVVfqk=",
        "ookrbATYovG3tEOXIH4HqWnsv8TrUlRWzM8AlCmW2mk=",
    ] {
        files.push(name, vec![0]).unwrap();
    }
    assert_eq!(files.len(), 3);
}

#[test]
fn push_rejects_names_that_are_not_plain_file_names() {
    for name in ["", ".", "..", "a/b", "a\\b", "../x", "..\\x", "C:x", "a:b", "x\0y"] {
        let mut files = SaveFiles::new();
        let error = files.push(name, vec![1]).unwrap_err();
        assert!(
            matches!(&error, FormatError::InvalidFileName { name: given } if given == name),
            "{name:?}: {error:?}"
        );
        assert!(files.is_empty());
    }
}

#[test]
fn push_rejects_an_exact_duplicate_and_keeps_case_variants() {
    let mut files = SaveFiles::new();
    files.push("A", vec![1]).unwrap();
    let error = files.push("A", vec![2]).unwrap_err();
    assert!(matches!(error, FormatError::DuplicateFile { name } if name == "A"));
    files.push("a", vec![3]).unwrap();
    assert_eq!(files.get("A"), Some(&[1][..]));
    assert_eq!(files.get("a"), Some(&[3][..]));
}

#[test]
fn a_rejected_push_changes_nothing() {
    let mut files = three();
    let before = files.clone();
    files.push("a", vec![9]).unwrap_err();
    files.push("x/y", vec![9]).unwrap_err();
    assert_eq!(files.len(), 3);
    assert_eq!(files, before);
    assert_eq!(files.iter().map(|(name, _)| name).collect::<Vec<_>>(), ["b", "a", "c"]);
    assert_eq!(files.get("a"), Some(&[2, 2][..]));
}

#[test]
fn get_and_iter_follow_insertion_order() {
    let files = three();
    let listed: Vec<(&str, &[u8])> = files.iter().collect();
    assert_eq!(listed, [("b", &[1][..]), ("a", &[2, 2][..]), ("c", &[][..])]);
    assert_eq!(files.get("b"), Some(&[1][..]));
    assert_eq!(files.get("c"), Some(&[][..]));
    assert_eq!(files.get("zzz"), None);
}

#[test]
fn an_empty_file_is_a_file() {
    let mut files = SaveFiles::new();
    assert!(files.is_empty());
    files.push("e", vec![]).unwrap();
    assert_eq!(files.get("e"), Some(&[][..]));
    assert!(!files.is_empty());
}

#[test]
fn debug_shows_names_and_counts_not_bytes() {
    let mut files = SaveFiles::new();
    files.push("slot.sav", b"SECRET-5c1e".repeat(3)).unwrap();
    let shown = format!("{files:?}");
    assert!(shown.contains("slot.sav"));
    assert!(shown.contains("33"));
    assert!(!shown.contains("SECRET-5c1e"));
    let decimal: Vec<String> = b"SECRET-5c1e".iter().map(u8::to_string).collect();
    assert!(!shown.contains(&decimal.join(", ")));
}

#[test]
fn files_equal_when_names_and_bytes_equal_in_order() {
    assert_eq!(three(), three());
    let mut other = SaveFiles::new();
    other.push("a", vec![2, 2]).unwrap();
    other.push("b", vec![1]).unwrap();
    other.push("c", vec![]).unwrap();
    assert_ne!(three(), other);
    let mut changed = three();
    changed.push("d", vec![]).unwrap();
    assert_ne!(three(), changed);
}

#[test]
fn probe_keeps_at_most_the_head_limit() {
    assert_eq!(PROBE_HEAD_LEN, 256);
    let long = vec![7u8; 300];
    assert_eq!(FileProbe::new("x", 300, &long).head.len(), PROBE_HEAD_LEN);
    assert_eq!(FileProbe::new("x", 10, &long[..10]).head.len(), 10);
    assert_eq!(FileProbe::new("x", 0, &[]).head.len(), 0);
    let probe = FileProbe::new("x", 5_000, &long[..10]);
    assert_eq!(probe.len, 5_000);
    assert_eq!(probe.name, "x");
}

#[test]
fn probe_keeps_the_first_bytes() {
    let bytes: Vec<u8> = (0..=255u8).chain(0..44).collect();
    let probe = FileProbe::new("x", 300, &bytes);
    assert_eq!(probe.head, &bytes[..PROBE_HEAD_LEN]);
}

#[test]
fn confidence_orders_possible_below_likely_below_certain() {
    assert!(Confidence::Possible < Confidence::Likely);
    assert!(Confidence::Likely < Confidence::Certain);
    let mut levels = [Confidence::Certain, Confidence::Possible, Confidence::Likely];
    levels.sort();
    assert_eq!(levels, [Confidence::Possible, Confidence::Likely, Confidence::Certain]);
}

#[test]
fn detection_new_has_no_slot_and_takes_assignment() {
    let mut detection = Detection::new(Confidence::Likely);
    assert_eq!(detection.slot, None);
    assert_eq!(detection.confidence, Confidence::Likely);
    detection.slot = Some(SaveSlot::Auto);
    assert_eq!(detection.slot, Some(SaveSlot::Auto));
}

fn assert_send_sync_static<T: Send + Sync + 'static>() {}

#[test]
fn files_and_probe_types_are_send_sync_and_static() {
    assert_send_sync_static::<SaveFiles>();
    assert_send_sync_static::<Detection>();
    assert_send_sync_static::<Confidence>();
    assert_send_sync_static::<FileProbe<'static>>();
}
