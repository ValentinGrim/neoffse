//! Edits through a `Document`: what changes, and what stays as read.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{changed_span, embed};
use neoffse_format_pr::container;
use neoffse_format_pr::document::{Document, DocumentError, Scalar, Segment};

fn parse(text: &str) -> Document {
    Document::parse(text).unwrap()
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
fn an_edit_re_escapes_untouched_siblings_inside_the_edited_string() {
    // The `\/` sits in the string token of `u`, not in the document it holds.
    let text = r#"{"u":"{\"a\":\"é\/x\",\"b\":1}"}"#;
    let mut document = parse(text);
    document.set(&[Segment::Key("u"), Segment::Doc, Segment::Key("b")], &Scalar::Int(2)).unwrap();
    let output = document.to_json();
    assert_eq!(output, format!("{{\"u\":{}}}", embed("{\"a\":\"é/x\",\"b\":2}")));
    let path = [Segment::Key("u"), Segment::Doc, Segment::Key("a")];
    assert_eq!(parse(&output).get(&path).unwrap(), Scalar::Text("é/x".to_owned()));

    // An escape written inside the embedded document itself (`\\/` in the token)
    // belongs to an untouched member and survives.
    let text = r#"{"u":"{\"a\":\"é\\/x\",\"b\":1}"}"#;
    let mut document = parse(text);
    document.set(&[Segment::Key("u"), Segment::Doc, Segment::Key("b")], &Scalar::Int(2)).unwrap();
    assert_eq!(document.to_json(), text.replace(":1}", ":2}"));
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
