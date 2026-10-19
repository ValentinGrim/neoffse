//! Parsing, reading and navigating a `Document`. No test here edits.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::embed;
use neoffse_format_pr::document::{Document, DocumentError, MAX_DEPTH, Scalar, Segment, render_path};

fn parse(text: &str) -> Document {
    Document::parse(text).unwrap()
}

fn get(text: &str, path: &[Segment<'_>]) -> Result<Scalar, DocumentError> {
    parse(text).get(path)
}

fn raw(text: &str, path: &[Segment<'_>]) -> String {
    parse(text).raw(path).unwrap().into_owned()
}

fn three_levels() -> String {
    let innermost = "{\"hp\":7}";
    let middle = format!("{{\"target\":[{}]}}", embed(innermost));
    format!("{{\"a\":{}}}", embed(&middle))
}

const THREE_LEVEL_PATH: [Segment<'static>; 5] = [Segment::Key("a"), Segment::Doc, Segment::Key("target"), Segment::Index(0), Segment::Doc];

#[test]
fn to_json_returns_the_input_for_every_form() {
    for text in [
        "{\"a\":1E5,\"b\":\"x\\/y\",\"c\":\"é\",\"d\":[ 1 , 2 ],\"e\":-0,\"f\":1.50}",
        "{\"a\":1,\"a\":2,\"b\":{\"c\":3,\"c\":4}}",
        "[1, 2 ,3]",
        "\"a\\/b\"",
        "1E5",
        "-0.0",
        "null",
    ] {
        assert_eq!(parse(text).to_json(), text);
    }
}

#[test]
fn outer_whitespace_survives() {
    let text = " \r\n{\"a\":1} \t\n";
    let mut document = parse(text);
    assert_eq!(document.to_json(), text);
    document.set(&[Segment::Key("a")], &Scalar::Int(2)).unwrap();
    assert_eq!(document.to_json(), " \r\n{\"a\":2} \t\n");
}

#[test]
fn parse_rejects_bad_text() {
    for text in ["", " \t\r\n", "{\"a\":1}{\"b\":2}", "{\"a\":", "\u{feff}{}", "}"] {
        match Document::parse(text) {
            Err(DocumentError::InvalidJson { path, .. }) => assert_eq!(path, "<root>"),
            other => panic!("{text:?} gave {other:?}"),
        }
    }
}

#[test]
fn raw_returns_the_text_as_written() {
    assert_eq!(raw("1E5", &[]), "1E5");
    assert_eq!(raw("58.0", &[]), "58.0");
    assert_eq!(raw("\"a\\/b\"", &[]), "\"a\\/b\"");
    assert_eq!(raw("{\"x\": [1, 2] ,\"y\":1E5}", &[Segment::Key("x")]), "[1, 2]");
    assert_eq!(raw("{\"x\": [1, 2] ,\"y\":1E5}", &[Segment::Key("y")]), "1E5");
}

#[test]
fn get_classifies_the_scalar_kinds() {
    let text = "{\"n\":null,\"t\":true,\"f\":false,\"i\":-12,\"x\":0.5,\"s\":\"hi\"}";
    assert_eq!(get(text, &[Segment::Key("n")]).unwrap(), Scalar::Null);
    assert_eq!(get(text, &[Segment::Key("t")]).unwrap(), Scalar::Bool(true));
    assert_eq!(get(text, &[Segment::Key("f")]).unwrap(), Scalar::Bool(false));
    assert_eq!(get(text, &[Segment::Key("i")]).unwrap(), Scalar::Int(-12));
    assert_eq!(get(text, &[Segment::Key("x")]).unwrap(), Scalar::Float(0.5));
    assert_eq!(get(text, &[Segment::Key("s")]).unwrap(), Scalar::Text("hi".to_owned()));
}

#[test]
fn get_reads_a_u32_above_i32_as_int() {
    assert_eq!(get("4294967295", &[]).unwrap(), Scalar::Int(4_294_967_295));
    assert_eq!(get("4293655417", &[]).unwrap(), Scalar::Int(4_293_655_417));
}

#[test]
fn get_reads_an_integer_above_2_pow_53_exactly() {
    assert_eq!(get("639278367168195674", &[]).unwrap(), Scalar::Int(639_278_367_168_195_674));
    assert_eq!(get("18446744073709551615", &[]).unwrap(), Scalar::Uint(u64::MAX));
}

#[test]
fn get_keeps_58_point_0_a_float() {
    let mut document = parse("58.0");
    assert_eq!(document.get(&[]).unwrap(), Scalar::Float(58.0));
    assert_eq!(document.raw(&[]).unwrap(), "58.0");
}

#[test]
fn get_of_a_huge_integer_is_out_of_range_but_raw_works() {
    let huge = "123456789012345678901234567890";
    let mut document = parse(huge);
    assert!(matches!(document.get(&[]), Err(DocumentError::NumberOutOfRange { .. })));
    assert_eq!(document.raw(&[]).unwrap(), huge);
}

#[test]
fn get_of_an_overflowing_exponent_is_out_of_range() {
    assert!(matches!(get("1e999", &[]), Err(DocumentError::NumberOutOfRange { .. })));
}

#[test]
fn get_of_a_container_is_a_wrong_kind() {
    for text in ["{\"a\":1}", "[1]"] {
        assert!(matches!(get(text, &[]), Err(DocumentError::WrongKind { expected: "scalar", .. })));
    }
}

#[test]
fn get_unescapes_a_string() {
    assert_eq!(get("\"a\\\"b\\\\cé\"", &[]).unwrap(), Scalar::Text("a\"b\\cé".to_owned()));
}

#[test]
fn a_lone_surrogate_string_is_an_error_not_a_panic() {
    let mut document = parse("\"\\ud800\"");
    assert_eq!(document.raw(&[]).unwrap(), "\"\\ud800\"");
    assert!(matches!(document.get(&[]), Err(DocumentError::InvalidJson { .. })));
    let mut object = parse("{\"\\ud800\":1}");
    assert!(matches!(object.raw(&[Segment::Key("x")]), Err(DocumentError::InvalidJson { .. })));
}

#[test]
fn doc_descends_into_a_string_document() {
    let path = [Segment::Key("userData"), Segment::Doc, Segment::Key("gil")];
    assert_eq!(get("{\"userData\":\"{\\\"gil\\\":5}\"}", &path).unwrap(), Scalar::Int(5));
}

#[test]
fn doc_descends_through_three_levels_and_arrays() {
    let path = [
        Segment::Key("a"),
        Segment::Doc,
        Segment::Key("target"),
        Segment::Index(0),
        Segment::Doc,
        Segment::Key("hp"),
    ];
    assert_eq!(get(&three_levels(), &path).unwrap(), Scalar::Int(7));
}

#[test]
fn doc_descends_into_a_string_document_that_is_an_array() {
    let text = format!("{{\"t\":{}}}", embed("[4,5]"));
    let path = [Segment::Key("t"), Segment::Doc, Segment::Index(1)];
    assert_eq!(get(&text, &path).unwrap(), Scalar::Int(5));
}

#[test]
fn missing_key_names_the_path_and_the_key() {
    let mut path = THREE_LEVEL_PATH.to_vec();
    path.push(Segment::Key("mp"));
    let error = get(&three_levels(), &path).unwrap_err();
    let message = error.to_string();
    match error {
        DocumentError::MissingKey { path, key } => {
            assert_eq!(path, "a.target[0]");
            assert_eq!(key, "mp");
        }
        other => panic!("{other:?}"),
    }
    assert!(message.contains("a.target[0]") && message.contains("mp"), "{message}");
}

#[test]
fn index_out_of_range_reports_the_length() {
    match get("{\"a\":[1,2,3]}", &[Segment::Key("a"), Segment::Index(3)]).unwrap_err() {
        DocumentError::IndexOutOfRange { path, index, len } => {
            assert_eq!((path.as_str(), index, len), ("a", 3, 3));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_step_on_the_wrong_kind_is_a_wrong_kind() {
    let cases: [(&str, Segment<'_>, &str); 5] = [
        ("[1]", Segment::Key("a"), "object"),
        ("{\"a\":1}", Segment::Index(0), "array"),
        ("12", Segment::Doc, "string"),
        ("{\"a\":1}", Segment::Doc, "string"),
        ("\"x\"", Segment::Key("a"), "object"),
    ];
    for (text, segment, wanted) in cases {
        match get(text, &[segment]) {
            Err(DocumentError::WrongKind { expected, .. }) => assert_eq!(expected, wanted, "{text}"),
            other => panic!("{text} gave {other:?}"),
        }
    }
}

#[test]
fn doc_on_a_plain_string_is_not_a_document() {
    for content in ["hello", "12", " {}"] {
        let text = format!("{{\"t\":{}}}", embed(content));
        let result = parse(&text).raw(&[Segment::Key("t"), Segment::Doc]).map(|_| ());
        assert!(matches!(result, Err(DocumentError::NotADocument { .. })), "{content:?}");
    }
}

#[test]
fn doc_on_a_document_with_trailing_whitespace_is_not_a_document() {
    let text = format!("{{\"t\":{}}}", embed("{} "));
    let result = parse(&text).raw(&[Segment::Key("t"), Segment::Doc]).map(|_| ());
    assert!(matches!(result, Err(DocumentError::NotADocument { .. })));
}

#[test]
fn doc_on_invalid_json_in_a_string_is_invalid_json() {
    let text = format!("{{\"t\":{}}}", embed("{bad"));
    match parse(&text).raw(&[Segment::Key("t"), Segment::Doc]).map(|_| ()) {
        Err(DocumentError::InvalidJson { path, .. }) => assert_eq!(path, "t"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn doc_on_an_empty_string_is_an_empty_document() {
    let text = "{\"t\":\"\"}";
    let mut document = parse(text);
    match document.raw(&[Segment::Key("t"), Segment::Doc]).map(|_| ()) {
        Err(DocumentError::EmptyDocument { path }) => assert_eq!(path, "t"),
        other => panic!("{other:?}"),
    }
    assert_eq!(document.to_json(), text);
}

#[test]
fn a_duplicate_key_is_refused_and_survives() {
    let text = "{\"a\":1,\"a\":2,\"b\":3}";
    let mut document = parse(text);
    assert!(matches!(document.get(&[Segment::Key("a")]), Err(DocumentError::DuplicateKey { .. })));
    assert_eq!(document.get(&[Segment::Key("b")]).unwrap(), Scalar::Int(3));
    assert_eq!(document.to_json(), text);
}

#[test]
fn a_path_over_the_limit_is_too_deep() {
    let too_long = vec![Segment::Index(0); MAX_DEPTH + 1];
    let mut document = parse("[1]");
    assert!(matches!(document.raw(&too_long), Err(DocumentError::TooDeep { limit: 64 })));
    assert!(matches!(document.get(&too_long), Err(DocumentError::TooDeep { limit: 64 })));
    assert!(matches!(document.set(&too_long, &Scalar::Null), Err(DocumentError::TooDeep { limit: 64 })));
    assert!(matches!(document.set_raw(&too_long, "1"), Err(DocumentError::TooDeep { limit: 64 })));

    let nested = format!("{}1{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
    let path = vec![Segment::Index(0); MAX_DEPTH];
    assert_eq!(get(&nested, &path).unwrap(), Scalar::Int(1));
}

#[test]
fn deep_nesting_does_not_overflow_the_stack() {
    let text = format!("{}{}", "[".repeat(100_000), "]".repeat(100_000));
    match Document::parse(&text) {
        Ok(mut document) => {
            assert!(document.to_json() == text, "deep nesting changed the text");
            let _ = document.get(&[Segment::Index(0)]);
        }
        Err(DocumentError::InvalidJson { .. }) => {}
        Err(other) => panic!("{other:?}"),
    }
}

#[test]
fn render_path_formats() {
    assert_eq!(render_path(&[]), "<root>");
    assert_eq!(render_path(&[Segment::Doc]), "<root>");
    assert_eq!(render_path(&[Segment::Index(3)]), "[3]");
    let path = [
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("ownedCharacterList"),
        Segment::Doc,
        Segment::Key("target"),
        Segment::Index(2),
        Segment::Doc,
        Segment::Key("parameter"),
    ];
    assert_eq!(render_path(&path), "userData.ownedCharacterList.target[2].parameter");
}

#[test]
fn document_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Document>();
    assert_send_sync::<DocumentError>();
}
