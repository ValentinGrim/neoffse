//! Failed row operations: the error shape, and bytes that stay put.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::embed;
use neoffse_format_pr::document::{Document, DocumentError, MAX_DEPTH, Scalar, Segment};

type Expect = fn(&DocumentError) -> bool;

const LIST: [Segment<'static>; 1] = [Segment::Key("list")];

/// Spaces after every colon and comma: a dirty mark would compact them.
fn spaced() -> String {
    let inner = embed(r#"{ "arr": [ 1, 2 ], "n": 1 }"#);
    format!(
        r#"{{ "list": [ 1, 2 ], "obj": {{ "a": 1 }}, "num": 5, "text": "hi", "nil": null, "doc": {inner}, "dup": {{ "k": 1, "k": 2 }}, "empty": "", "plain": "abc" }}"#
    )
}

fn doc() -> (Document, String) {
    let text = spaced();
    (Document::parse(&text).unwrap(), text)
}

#[test]
fn insert_past_the_end_is_index_out_of_range() {
    let (mut document, text) = doc();
    let err = document.insert(&LIST, 3, &Scalar::Int(1)).unwrap_err();
    assert!(matches!(err, DocumentError::IndexOutOfRange { index: 3, len: 2, .. }), "{err:?}");
    assert!(err.to_string().contains("list"), "{err}");
    assert_eq!(document.to_json(), text);
}

#[test]
fn remove_at_len_and_from_an_empty_array_are_index_out_of_range() {
    let (mut document, text) = doc();
    let err = document.remove(&LIST, 2).unwrap_err();
    assert!(matches!(err, DocumentError::IndexOutOfRange { index: 2, len: 2, .. }), "{err:?}");
    assert_eq!(document.to_json(), text);

    let mut empty = Document::parse(r#"{ "list": [ ] }"#).unwrap();
    let err = empty.remove(&LIST, 0).unwrap_err();
    assert!(matches!(err, DocumentError::IndexOutOfRange { index: 0, len: 0, .. }), "{err:?}");
    assert_eq!(empty.to_json(), r#"{ "list": [ ] }"#);
}

#[test]
fn a_failed_row_edit_leaves_every_level_clean() {
    let (mut document, text) = doc();
    let deep = [Segment::Key("doc"), Segment::Doc, Segment::Key("arr")];
    let not_array = [Segment::Key("doc"), Segment::Doc, Segment::Key("n")];
    for path in [&deep[..], &not_array[..]] {
        assert!(document.insert(path, 9, &Scalar::Int(1)).is_err());
        assert!(document.remove(path, 9).is_err());
        assert_eq!(document.to_json(), text);
    }
    assert!(document.insert_raw(&deep, 0, "{").is_err());
    assert!(document.insert(&deep, 0, &Scalar::Float(f64::NAN)).is_err());
    assert!(document.remove(&deep, 2).is_err());
    assert!(document.insert(&[Segment::Key("nope")], 0, &Scalar::Int(1)).is_err());
    assert_eq!(document.to_json(), text);
    // The control: a success compacts, which the failures above did not.
    document.insert(&deep, 0, &Scalar::Int(0)).unwrap();
    assert_ne!(document.to_json(), text);
}

#[test]
fn a_path_that_is_not_an_array_is_wrong_kind() {
    let (mut document, text) = doc();
    for key in ["obj", "num", "text", "nil"] {
        let path = [Segment::Key(key)];
        let errors = [
            document.array_len(&path).unwrap_err(),
            document.insert(&path, 0, &Scalar::Int(1)).unwrap_err(),
            document.remove(&path, 0).unwrap_err(),
        ];
        for err in errors {
            assert!(matches!(err, DocumentError::WrongKind { expected: "array", .. }), "{key}: {err:?}");
        }
        assert_eq!(document.to_json(), text);
    }
}

#[test]
fn step_errors_surface_unchanged() {
    let (mut document, text) = doc();
    let cases: [(Vec<Segment<'static>>, Expect); 5] = [
        (vec![Segment::Key("nope")], |e| matches!(e, DocumentError::MissingKey { .. })),
        (vec![Segment::Key("dup"), Segment::Key("k")], |e| {
            matches!(e, DocumentError::DuplicateKey { .. })
        }),
        (vec![Segment::Key("list"), Segment::Index(5)], |e| {
            matches!(e, DocumentError::IndexOutOfRange { index: 5, len: 2, .. })
        }),
        (vec![Segment::Key("empty"), Segment::Doc], |e| matches!(e, DocumentError::EmptyDocument { .. })),
        (vec![Segment::Key("plain"), Segment::Doc], |e| matches!(e, DocumentError::NotADocument { .. })),
    ];
    for (path, expect) in cases {
        let errors = [
            document.array_len(&path).unwrap_err(),
            document.insert(&path, 0, &Scalar::Int(1)).unwrap_err(),
            document.remove(&path, 0).unwrap_err(),
        ];
        for err in errors {
            assert!(expect(&err), "{path:?}: {err:?}");
        }
        assert_eq!(document.to_json(), text);
    }
}

#[test]
fn insert_raw_with_invalid_json_is_invalid_json() {
    let (mut document, text) = doc();
    for json in ["", "{", "[1,", "1 2", "nope"] {
        let err = document.insert_raw(&LIST, 0, json).unwrap_err();
        assert!(matches!(err, DocumentError::InvalidJson { .. }), "{json:?}: {err:?}");
    }
    assert_eq!(document.to_json(), text);
}

#[test]
fn insert_of_nan_is_non_finite_float() {
    let (mut document, text) = doc();
    for value in [f64::NAN, f64::INFINITY] {
        let err = document.insert(&LIST, 0, &Scalar::Float(value)).unwrap_err();
        assert!(matches!(err, DocumentError::NonFiniteFloat { .. }), "{err:?}");
    }
    assert_eq!(document.to_json(), text);
}

#[test]
fn a_non_finite_value_is_reported_before_a_path_that_does_not_exist() {
    let (mut document, text) = doc();
    let err = document.insert(&[Segment::Key("nope")], 99, &Scalar::Float(f64::NAN)).unwrap_err();
    assert!(matches!(err, DocumentError::NonFiniteFloat { .. }), "{err:?}");
    assert_eq!(document.to_json(), text);
}

#[test]
fn an_invalid_json_argument_wins_over_a_bad_path() {
    let (mut document, _) = doc();
    let err = document.insert_raw(&[Segment::Key("nope")], 99, "{").unwrap_err();
    assert!(matches!(err, DocumentError::InvalidJson { .. }), "{err:?}");
}

#[test]
fn too_deep_paths_are_refused() {
    let (mut document, text) = doc();
    let path = vec![Segment::Doc; MAX_DEPTH + 1];
    let is_deep = |e: DocumentError| matches!(e, DocumentError::TooDeep { limit } if limit == MAX_DEPTH);
    assert!(is_deep(document.array_len(&path).unwrap_err()));
    assert!(is_deep(document.insert(&path, 0, &Scalar::Int(1)).unwrap_err()));
    assert!(is_deep(document.insert_raw(&path, 0, "1").unwrap_err()));
    assert!(is_deep(document.remove(&path, 0).unwrap_err()));
    assert_eq!(document.to_json(), text);
}
