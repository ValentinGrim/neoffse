//! Failed parses and failed sets: the error shape, and bytes that stay put.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::error::Error;

use common::embed;
use neoffse_format_pr::document::{Document, DocumentError, MAX_DEPTH, Scalar, Segment};

/// What a failing call writes: a scalar through `set`, or text through `set_raw`.
enum Write {
    Set(Scalar),
    Raw(&'static str),
}

type Expect = fn(&DocumentError) -> bool;

struct Failure {
    path: Vec<Segment<'static>>,
    write: Write,
    expect: Expect,
}

fn fail(path: &[Segment<'static>], write: Write, expect: Expect) -> Failure {
    Failure {
        path: path.to_vec(),
        write,
        expect,
    }
}

/// Pretty-printed at every level, so a re-serialization changes bytes.
fn pretty_text() -> String {
    let deep = "{ \"v\": 1, \"w\": [ 1, 2 ] }";
    let inner = format!(
        "{{ \"nested\": {}, \"empty\": \"\", \"plain\": \"hello\", \"arr\": [ 1, 2 ], \"num\": 42, \
         \"dup\": {{ \"k\": 1, \"k\": 2 }} }}",
        embed(deep)
    );
    format!("{{\n  \"userData\": {},\n  \"other\": 1\n}}\n", embed(&inner))
}

fn failures() -> Vec<Failure> {
    use Segment::{Doc, Index, Key};
    let top = [Key("userData"), Doc];
    let under = |tail: &[Segment<'static>]| [&top[..], tail].concat();
    let too_deep = [&top[..], &vec![Index(0); MAX_DEPTH - 1]].concat();
    vec![
        fail(&under(&[Key("nope")]), Write::Set(Scalar::Int(1)), |e| {
            matches!(e, DocumentError::MissingKey { .. })
        }),
        fail(&under(&[Key("nested"), Doc, Key("zz")]), Write::Raw("1"), |e| {
            matches!(e, DocumentError::MissingKey { .. })
        }),
        fail(&under(&[Key("arr"), Index(5)]), Write::Raw("1"), |e| {
            matches!(e, DocumentError::IndexOutOfRange { .. })
        }),
        fail(&under(&[Key("nested"), Doc, Key("w"), Index(2)]), Write::Set(Scalar::Int(1)), |e| {
            matches!(e, DocumentError::IndexOutOfRange { .. })
        }),
        fail(&under(&[Key("num"), Key("x")]), Write::Set(Scalar::Int(1)), |e| {
            matches!(e, DocumentError::WrongKind { .. })
        }),
        fail(&under(&[Key("arr"), Key("x")]), Write::Raw("1"), |e| {
            matches!(e, DocumentError::WrongKind { .. })
        }),
        fail(&under(&[Key("plain"), Doc, Key("x")]), Write::Set(Scalar::Int(1)), |e| {
            matches!(e, DocumentError::NotADocument { .. })
        }),
        fail(&under(&[Key("empty"), Doc, Key("x")]), Write::Raw("1"), |e| {
            matches!(e, DocumentError::EmptyDocument { .. })
        }),
        fail(&under(&[Key("dup"), Key("k")]), Write::Set(Scalar::Int(3)), |e| {
            matches!(e, DocumentError::DuplicateKey { .. })
        }),
        fail(&under(&[Key("nested"), Doc]), Write::Raw("{}"), |e| {
            matches!(e, DocumentError::EndsInDoc { .. })
        }),
        fail(&too_deep, Write::Set(Scalar::Int(1)), |e| matches!(e, DocumentError::TooDeep { .. })),
        fail(&under(&[Key("num")]), Write::Raw("{bad"), |e| matches!(e, DocumentError::InvalidJson { .. })),
        fail(&under(&[Key("nested"), Doc, Key("v")]), Write::Raw("[1,"), |e| {
            matches!(e, DocumentError::InvalidJson { .. })
        }),
        fail(&under(&[Key("num")]), Write::Set(Scalar::Float(f64::NAN)), |e| {
            matches!(e, DocumentError::NonFiniteFloat { .. })
        }),
        fail(&under(&[Key("nested"), Doc, Key("v")]), Write::Set(Scalar::Float(f64::INFINITY)), |e| {
            matches!(e, DocumentError::NonFiniteFloat { .. })
        }),
    ]
}

fn run_all(document: &mut Document, text: &str, label: &str) {
    for (index, failure) in failures().iter().enumerate() {
        let result = match &failure.write {
            Write::Set(value) => document.set(&failure.path, value),
            Write::Raw(json) => document.set_raw(&failure.path, json),
        };
        match result {
            Err(error) => assert!((failure.expect)(&error), "{label} case {index}: {error:?}"),
            Ok(()) => panic!("{label} case {index}: the call succeeded"),
        }
        assert!(document.to_json() == text, "{label} case {index}: the bytes changed");
    }
}

#[test]
fn a_failed_set_through_string_documents_changes_no_byte() {
    use Segment::{Doc, Key};
    let text = pretty_text();

    let mut fresh = Document::parse(&text).unwrap();
    run_all(&mut fresh, &text, "fresh");

    // Reads parse and cache every level the failing paths cross.
    let mut cached = Document::parse(&text).unwrap();
    let reads: [&[Segment<'_>]; 4] = [
        &[Key("userData"), Doc, Key("num")],
        &[Key("userData"), Doc, Key("nested"), Doc, Key("v")],
        &[Key("userData"), Doc, Key("arr"), Segment::Index(1)],
        &[Key("userData"), Doc, Key("plain")],
    ];
    for path in reads {
        cached.get(path).unwrap();
    }
    assert!(cached.to_json() == text, "the reads changed the bytes");
    run_all(&mut cached, &text, "cached");
}

#[test]
fn invalid_json_display_omits_the_source_and_source_is_set() {
    let error = Document::parse("{bad").unwrap_err();
    assert_eq!(error.to_string(), "invalid JSON at <root>");
    assert!(matches!(error, DocumentError::InvalidJson { .. }));
    assert!(error.source().is_some());
}

#[test]
fn parse_error_positions_count_from_the_first_byte() {
    let Err(DocumentError::InvalidJson { source, .. }) = Document::parse("\n\n{bad") else {
        panic!("expected InvalidJson");
    };
    assert_eq!(source.line(), 3);
}

#[test]
fn set_raw_error_positions_count_from_the_first_byte() {
    let mut document = Document::parse("[0]").unwrap();
    let Err(DocumentError::InvalidJson { source, .. }) = document.set_raw(&[Segment::Index(0)], "\n\n{bad") else {
        panic!("expected InvalidJson");
    };
    assert_eq!(source.line(), 3);
    assert_eq!(document.to_json(), "[0]");
}
