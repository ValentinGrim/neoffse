//! Scalars written by `set` and `set_raw`: the text each one produces.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_format_pr::document::{Document, DocumentError, Scalar, Segment};

fn parse(text: &str) -> Document {
    Document::parse(text).unwrap()
}

fn raw_of(document: &mut Document, path: &[Segment<'_>]) -> String {
    document.raw(path).unwrap().into_owned()
}

#[test]
fn set_text_escapes_like_serde_json() {
    let cases = ["a/b", "é", "😀", "\u{2028}", "\u{7f}", "\"", "\\", "\u{1f}", "\n", ""];
    for case in cases {
        let mut document = parse("{\"t\":0}");
        document.set(&[Segment::Key("t")], &Scalar::Text(case.to_owned())).unwrap();
        let written = raw_of(&mut document, &[Segment::Key("t")]);
        assert_eq!(written, serde_json::to_string(case).unwrap(), "{case:?}");
        assert_eq!(parse(&document.to_json()).get(&[Segment::Key("t")]).unwrap(), Scalar::Text(case.to_owned()));
    }
    for (case, literal) in [("\u{1f}", "\"\\u001f\""), ("a/b", "\"a/b\"")] {
        let mut document = parse("[0]");
        document.set(&[Segment::Index(0)], &Scalar::Text(case.to_owned())).unwrap();
        assert_eq!(raw_of(&mut document, &[Segment::Index(0)]), literal);
    }
}

#[test]
fn set_float_keeps_the_decimal_point() {
    let cases = [
        (58.0, "58.0"),
        (1.0, "1.0"),
        (0.0, "0.0"),
        (-0.0, "-0.0"),
        (0.5, "0.5"),
        (12_345.678_901_234_567, "12345.678901234567"),
    ];
    for (float, text) in cases {
        let mut document = parse("[0]");
        document.set(&[Segment::Index(0)], &Scalar::Float(float)).unwrap();
        assert_eq!(raw_of(&mut document, &[Segment::Index(0)]), text);
        assert_eq!(document.to_json(), format!("[{text}]"));
    }
}

#[test]
fn set_float_rejects_nan_and_infinity() {
    let text = "{ \"a\" : 1 }";
    for float in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut document = parse(text);
        let result = document.set(&[Segment::Key("a")], &Scalar::Float(float));
        assert!(matches!(result, Err(DocumentError::NonFiniteFloat { .. })));
        assert_eq!(document.to_json(), text);
    }
}

#[test]
fn set_integers_are_plain_text() {
    let cases = [
        (Scalar::Int(i64::MIN), "-9223372036854775808"),
        (Scalar::Int(i64::MAX), "9223372036854775807"),
        (Scalar::Uint(u64::MAX), "18446744073709551615"),
        (Scalar::Int(4_294_967_295), "4294967295"),
        (Scalar::Int(0), "0"),
    ];
    for (value, text) in cases {
        let mut document = parse("[0]");
        document.set(&[Segment::Index(0)], &value).unwrap();
        assert_eq!(document.to_json(), format!("[{text}]"));
    }
}

#[test]
fn set_u32_above_i32_and_i64_above_2_pow_53_round_trip() {
    let values = [
        Scalar::Int(4_293_655_417),
        Scalar::Int(600_000_000_000_000_013),
        Scalar::Int(i64::MAX),
        Scalar::Int(i64::MIN),
        Scalar::Uint(u64::MAX),
        Scalar::Uint(i64::MAX as u64 + 1),
    ];
    for value in values {
        let mut document = parse("[0]");
        document.set(&[Segment::Index(0)], &value).unwrap();
        assert!(document.get(&[Segment::Index(0)]).unwrap() == value, "{value:?}");
    }
}

#[test]
fn set_bool_and_null() {
    let mut document = parse("{\"a\":1,\"b\":2}");
    document.set(&[Segment::Key("a")], &Scalar::Bool(true)).unwrap();
    document.set(&[Segment::Key("b")], &Scalar::Null).unwrap();
    assert_eq!(document.to_json(), "{\"a\":true,\"b\":null}");
    assert_eq!(document.get(&[Segment::Key("a")]).unwrap(), Scalar::Bool(true));
    assert_eq!(document.get(&[Segment::Key("b")]).unwrap(), Scalar::Null);
}

#[test]
fn set_raw_keeps_the_given_form() {
    let mut document = parse("{\"a\":0,\"target\":[1,2],\"c\":5}");
    document.set_raw(&[Segment::Key("a")], "1E5").unwrap();
    assert_eq!(raw_of(&mut document, &[Segment::Key("a")]), "1E5");
    document.set_raw(&[Segment::Key("target")], "[ 3 , 4 ]").unwrap();
    document.set_raw(&[Segment::Key("c")], " \t7\r\n ").unwrap();
    assert_eq!(document.to_json(), "{\"a\":1E5,\"target\":[ 3 , 4 ],\"c\":7}");
}

#[test]
fn set_raw_rejects_invalid_json() {
    let text = "{ \"a\" : 1 }";
    for json in ["{bad", "", "  ", "1 2", "[1,"] {
        let mut document = parse(text);
        let result = document.set_raw(&[Segment::Key("a")], json);
        assert!(matches!(result, Err(DocumentError::InvalidJson { .. })), "{json:?}");
        assert_eq!(document.to_json(), text);
    }
}
#[test]
fn set_float_outside_the_plain_range_uses_an_exponent() {
    for (float, text) in [(1e16, "1e+16"), (1e-7, "1e-7")] {
        let mut document = parse("[0]");
        document.set(&[Segment::Index(0)], &Scalar::Float(float)).unwrap();
        assert_eq!(raw_of(&mut document, &[Segment::Index(0)]), text);
        assert_eq!(document.to_json(), format!("[{text}]"));
    }
}
