//! Edits through a `Document`: what changes, and what stays as read.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_format_pr::container;
use neoffse_format_pr::document::{Document, DocumentError, Scalar, Segment};

/// Wraps `doc` in quotes and escapes `\` and `"`, as the game embeds a document.
fn embed(doc: &str) -> String {
    format!("\"{}\"", doc.replace('\\', "\\\\").replace('"', "\\\""))
}

fn parse(text: &str) -> Document {
    Document::parse(text).unwrap()
}

/// What remains of `a` and `b` once the common prefix and suffix are gone.
fn changed_span<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    let mut prefix = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    while !(a.is_char_boundary(prefix) && b.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    let (a, b) = (&a[prefix..], &b[prefix..]);
    let mut suffix = a.bytes().rev().zip(b.bytes().rev()).take_while(|(x, y)| x == y).count();
    while !(a.is_char_boundary(a.len() - suffix) && b.is_char_boundary(b.len() - suffix)) {
        suffix -= 1;
    }
    (&a[..a.len() - suffix], &b[..b.len() - suffix])
}

/// `{"k":<level 1>,"z":3}` nested `depth` times around `{"x":100,"y":2}`;
/// returns the text and the path of `x`.
fn nested(depth: usize) -> (String, Vec<Segment<'static>>) {
    let mut text = "{\"x\":100,\"y\":2}".to_owned();
    let mut path = vec![Segment::Key("x")];
    for _ in 0..depth {
        text = format!("{{\"k\":{},\"z\":3}}", embed(&text));
        path.splice(0..0, [Segment::Key("k"), Segment::Doc]);
    }
    (text, path)
}

fn raw_of(document: &mut Document, path: &[Segment<'_>]) -> String {
    document.raw(path).unwrap().into_owned()
}

#[test]
fn reading_every_path_does_not_change_the_output() {
    let text = "{ \"a\" : 1E5 ,\n \"b\":\"x\\/y\", \"c\":\"{\\u0022k\\u0022:[1, 2]}\" }";
    let mut document = parse(text);
    let paths: [&[Segment<'_>]; 8] = [
        &[],
        &[Segment::Key("a")],
        &[Segment::Key("b")],
        &[Segment::Key("c")],
        &[Segment::Key("c"), Segment::Doc],
        &[Segment::Key("c"), Segment::Doc, Segment::Key("k")],
        &[Segment::Key("c"), Segment::Doc, Segment::Key("k"), Segment::Index(1)],
        &[Segment::Key("b")],
    ];
    for path in paths {
        document.raw(path).unwrap();
        document.get(path).ok();
    }
    assert_eq!(document.to_json(), text);
}

#[test]
fn an_edit_at_each_depth_changes_only_the_edited_digits() {
    for depth in 0..=3 {
        let (text, path) = nested(depth);
        let mut document = parse(&text);
        document.set(&path, &Scalar::Int(999)).unwrap();
        let output = document.to_json();
        assert_eq!(changed_span(&text, &output), ("100", "999"), "depth {depth}");
        assert_eq!(parse(&output).get(&path).unwrap(), Scalar::Int(999), "depth {depth}");
    }
}

#[test]
fn an_edit_keeps_the_forms_of_untouched_members() {
    let mut document = parse("{\"a\":1E5,\"b\":2,\"c\":\"x\\/y\",\"d\":58.0,\"e\":-0}");
    document.set(&[Segment::Key("b")], &Scalar::Int(3)).unwrap();
    assert_eq!(document.to_json(), "{\"a\":1E5,\"b\":3,\"c\":\"x\\/y\",\"d\":58.0,\"e\":-0}");
}

#[test]
fn an_edit_keeps_sibling_string_documents_verbatim() {
    let config = "\"{\\\"p\\\":\\\"a\\/b\\\",\\\"q\\\" : 1}\"";
    let text = format!("{{\"userData\":{},\"configData\":{config}}}", embed("{\"gil\":5}"));
    let mut document = parse(&text);
    document
        .set(&[Segment::Key("userData"), Segment::Doc, Segment::Key("gil")], &Scalar::Int(6))
        .unwrap();
    let expected = format!("{{\"userData\":{},\"configData\":{config}}}", embed("{\"gil\":6}"));
    assert_eq!(document.to_json(), expected);
}

#[test]
fn a_touched_level_is_compact_and_re_escapes_its_keys() {
    let mut document = parse("{ \"a\\/b\" : 1 , \"c\" : 2 }");
    document.set(&[Segment::Key("c")], &Scalar::Int(3)).unwrap();
    assert_eq!(document.to_json(), "{\"a/b\":1,\"c\":3}");
}

#[test]
fn an_edit_keeps_duplicate_keys_of_a_touched_level() {
    let mut document = parse("{\"a\":1,\"a\":2,\"b\":3}");
    document.set(&[Segment::Key("b")], &Scalar::Int(4)).unwrap();
    assert_eq!(document.to_json(), "{\"a\":1,\"a\":2,\"b\":4}");
}

#[test]
fn an_edit_re_escapes_quotes_and_backslashes_at_depth() {
    let value = "q\"b\\c\né";
    let (text, path) = nested(3);
    let mut document = parse(&text);
    document.set(&path, &Scalar::Text(value.to_owned())).unwrap();
    assert_eq!(document.get(&path).unwrap(), Scalar::Text(value.to_owned()));
    let output = document.to_json();
    assert_eq!(parse(&output).get(&path).unwrap(), Scalar::Text(value.to_owned()));

    let token = serde_json::to_string(value).unwrap();
    let mut expected = format!("{{\"x\":{token},\"y\":2}}");
    for _ in 0..3 {
        expected = format!("{{\"k\":{},\"z\":3}}", embed(&expected));
    }
    assert!(output == expected, "output differs from the layered escape");
    let run = format!("q{}\"b", "\\".repeat(15));
    assert!(output.contains(&run), "a quote at depth 3 needs 15 backslashes");
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
        (39_087.713_848_447_43, "39087.71384844743"),
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
        Scalar::Int(639_278_367_168_195_674),
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
fn a_failed_set_leaves_the_document_untouched() {
    let text = " { \"a\" : [ 1 , 2 ] , \"b\" : { \"c\" : 1 } } ";
    let mut document = parse(text);
    let failures: [(&[Segment<'_>], &str); 4] = [
        (&[Segment::Key("b"), Segment::Key("zz")], "1"),
        (&[Segment::Key("a"), Segment::Index(5)], "1"),
        (&[Segment::Key("b"), Segment::Key("c")], "{bad"),
        (&[Segment::Key("b"), Segment::Key("c"), Segment::Key("d")], "1"),
    ];
    for (path, json) in failures {
        assert!(document.set_raw(path, json).is_err());
        assert!(document.to_json() == text, "after a failed set_raw");
    }
    assert!(document.set(&[Segment::Key("a"), Segment::Index(9)], &Scalar::Int(1)).is_err());
    assert!(document.to_json() == text, "after a failed set");
}

#[test]
fn set_through_an_empty_document_fails_and_keeps_the_string() {
    let text = "{\"t\":\"\"}";
    let mut document = parse(text);
    let path = [Segment::Key("t"), Segment::Doc, Segment::Key("x")];
    assert!(matches!(document.set(&path, &Scalar::Int(1)), Err(DocumentError::EmptyDocument { .. })));
    assert_eq!(document.to_json(), text);
}

#[test]
fn set_text_over_an_empty_document_fills_it() {
    let mut document = parse("{\"t\":\"\"}");
    let doc_path = [Segment::Key("t"), Segment::Doc, Segment::Key("a")];
    assert!(document.get(&doc_path).is_err());
    document.set(&[Segment::Key("t")], &Scalar::Text("{\"a\":1}".to_owned())).unwrap();
    assert_eq!(document.to_json(), "{\"t\":\"{\\\"a\\\":1}\"}");
    assert_eq!(document.get(&doc_path).unwrap(), Scalar::Int(1));
}

#[test]
fn a_set_path_ending_in_doc_is_refused() {
    let text = "{\"t\":\"{}\"}";
    let mut document = parse(text);
    let path = [Segment::Key("t"), Segment::Doc];
    assert!(matches!(document.set_raw(&path, "{}"), Err(DocumentError::EndsInDoc { .. })));
    assert!(matches!(document.set(&path, &Scalar::Int(1)), Err(DocumentError::EndsInDoc { .. })));
    assert_eq!(document.to_json(), text);
}

#[test]
fn set_replaces_the_root() {
    let mut document = parse("{}");
    document.set_raw(&[], "[1]").unwrap();
    assert_eq!(document.to_json(), "[1]");
    let mut document = parse(" {} \n");
    document.set_raw(&[], "[1]").unwrap();
    assert_eq!(document.to_json(), " [1] \n");
    document.set(&[], &Scalar::Int(8)).unwrap();
    assert_eq!(document.to_json(), " 8 \n");
}

#[test]
fn two_edits_in_different_branches_and_last_edit_wins() {
    let mut document = parse("{\"a\":{\"x\":1,\"y\":2},\"b\":[1,2,3]}");
    let x = [Segment::Key("a"), Segment::Key("x")];
    document.set(&x, &Scalar::Int(10)).unwrap();
    document.set(&[Segment::Key("b"), Segment::Index(1)], &Scalar::Int(20)).unwrap();
    document.set(&x, &Scalar::Int(11)).unwrap();
    assert_eq!(document.to_json(), "{\"a\":{\"x\":11,\"y\":2},\"b\":[1,20,3]}");
}

#[test]
fn raw_and_get_after_an_edit_see_the_new_value() {
    let mut document = parse(&format!("{{\"s\":{}}}", embed("{\"n\":1,\"m\":2}")));
    let n = [Segment::Key("s"), Segment::Doc, Segment::Key("n")];
    document.set(&n, &Scalar::Int(9)).unwrap();
    assert_eq!(raw_of(&mut document, &[Segment::Key("s"), Segment::Doc]), "{\"n\":9,\"m\":2}");
    assert_eq!(document.get(&[Segment::Key("s")]).unwrap(), Scalar::Text("{\"n\":9,\"m\":2}".to_owned()));
    assert_eq!(raw_of(&mut document, &[]), "{\"s\":\"{\\\"n\\\":9,\\\"m\\\":2}\"}");
    assert_eq!(document.get(&n).unwrap(), Scalar::Int(9));
}

#[test]
fn committed_fixtures_round_trip() {
    for name in ["game-style.sav", "editor-style.sav", "large.sav", "non-ascii.sav", "no-bom.sav"] {
        let bytes = neoffse_testkit::read_fixture(&format!("pr/{name}")).unwrap();
        let decoded = container::decode(&bytes).unwrap();
        let document = Document::parse(&decoded.json).unwrap();
        assert!(document.to_json() == decoded.json, "{name}");
    }
}
