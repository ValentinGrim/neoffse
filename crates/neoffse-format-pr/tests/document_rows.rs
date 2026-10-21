//! Row operations of `Document`: insert, remove and the array length.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::embed;
use neoffse_format_pr::document::{Document, Scalar, Segment};

const LIST: [Segment<'static>; 1] = [Segment::Key("list")];

fn doc(text: &str) -> Document {
    Document::parse(text).unwrap()
}

fn elements(document: &mut Document, len: usize) -> Vec<String> {
    (0..len)
        .map(|index| document.raw(&[Segment::Key("list"), Segment::Index(index)]).unwrap().into_owned())
        .collect()
}

#[test]
fn array_len_counts_the_elements_at_any_depth() {
    let inner = embed(r#"{"target":[1,2,3,4]}"#);
    let mut document = doc(&format!(r#"{{"top":[1,[5,6]],"user":{inner}}}"#));
    assert_eq!(document.array_len(&[Segment::Key("top")]).unwrap(), 2);
    assert_eq!(document.array_len(&[Segment::Key("top"), Segment::Index(1)]).unwrap(), 2);
    let target = [Segment::Key("user"), Segment::Doc, Segment::Key("target")];
    assert_eq!(document.array_len(&target).unwrap(), 4);
}

#[test]
fn array_len_of_an_empty_array_is_zero() {
    let mut document = doc(r#"{ "list": [ ] }"#);
    assert_eq!(document.array_len(&LIST).unwrap(), 0);
    assert_eq!(document.to_json(), r#"{ "list": [ ] }"#);
}

#[test]
fn array_len_never_marks_a_level_dirty() {
    let text = r#"{ "list": [ 1, 2 ], "other": { "a": 1 } }"#;
    let mut document = doc(text);
    assert_eq!(document.array_len(&LIST).unwrap(), 2);
    assert_eq!(document.to_json(), text);
}

#[test]
fn insert_before_an_index_and_at_the_end() {
    let mut document = doc(r#"{"list":[1,2]}"#);
    document.insert(&LIST, 0, &Scalar::Int(0)).unwrap();
    document.insert(&LIST, 2, &Scalar::Int(9)).unwrap();
    document.insert(&LIST, 4, &Scalar::Int(7)).unwrap();
    assert_eq!(document.to_json(), r#"{"list":[0,1,9,2,7]}"#);
    assert_eq!(document.array_len(&LIST).unwrap(), 5);
}

#[test]
fn insert_into_an_empty_array() {
    let mut document = doc(r#"{"list":[]}"#);
    document.insert(&LIST, 0, &Scalar::Int(1)).unwrap();
    assert_eq!(document.to_json(), r#"{"list":[1]}"#);
}

#[test]
fn insert_keeps_the_text_of_every_sibling() {
    let mut document = doc(r#"{"list":[1.50,"a\/b",1e2]}"#);
    document.insert(&LIST, 1, &Scalar::Int(5)).unwrap();
    assert_eq!(elements(&mut document, 4), ["1.50", "5", r#""a\/b""#, "1e2"]);
}

#[test]
fn insert_compacts_only_the_levels_it_crosses() {
    let mut document = doc(r#"{ "side": { "a": 1 }, "list": [ 1, 2 ] }"#);
    document.insert(&LIST, 2, &Scalar::Int(3)).unwrap();
    assert_eq!(document.to_json(), r#"{"side":{ "a": 1 },"list":[1,2,3]}"#);
}

#[test]
fn insert_into_an_array_inside_a_string_document() {
    let inner = embed(r#"{"target":["a","b"]}"#);
    let mut document = doc(&format!(r#"{{ "keep": 1.0, "user": {inner} }}"#));
    let target = [Segment::Key("user"), Segment::Doc, Segment::Key("target")];
    document.insert(&target, 1, &Scalar::Text("x\"y".to_owned())).unwrap();
    let expected = embed(r#"{"target":["a","x\"y","b"]}"#);
    assert_eq!(document.to_json(), format!(r#"{{"keep":1.0,"user":{expected}}}"#));
}

#[test]
fn insert_scalar_text_writes_a_string_token() {
    let mut document = doc(r#"{"list":[]}"#);
    document.insert(&LIST, 0, &Scalar::Text(r#"{"a":1}"#.to_owned())).unwrap();
    assert_eq!(document.to_json(), r#"{"list":["{\"a\":1}"]}"#);
    let path = [Segment::Key("list"), Segment::Index(0)];
    assert_eq!(document.get(&path).unwrap(), Scalar::Text(r#"{"a":1}"#.to_owned()));
}

#[test]
fn insert_raw_keeps_the_inner_form_and_drops_outer_whitespace() {
    let mut document = doc(r#"{"list":[1]}"#);
    document.insert_raw(&LIST, 1, "\n [ 1,  2 ] \t").unwrap();
    assert_eq!(document.to_json(), r#"{"list":[1,[ 1,  2 ]]}"#);
}

#[test]
fn remove_returns_the_exact_text_of_the_element() {
    let mut document = doc(r#"{"list":[1.50,"a\/b\"c",2]}"#);
    assert_eq!(document.remove(&LIST, 0).unwrap(), "1.50");
    assert_eq!(document.remove(&LIST, 0).unwrap(), r#""a\/b\"c""#);
    assert_eq!(document.to_json(), r#"{"list":[2]}"#);
}

#[test]
fn remove_returns_the_rewritten_text_of_an_edited_element() {
    let mut document = doc(r#"{"list":[{ "a": 1, "b": 2 },3]}"#);
    let a = [Segment::Key("list"), Segment::Index(0), Segment::Key("a")];
    document.set(&a, &Scalar::Int(9)).unwrap();
    let raw = document.raw(&[Segment::Key("list"), Segment::Index(0)]).unwrap().into_owned();
    assert_eq!(raw, r#"{"a":9,"b":2}"#);
    assert_eq!(document.remove(&LIST, 0).unwrap(), raw);
}

#[test]
fn insert_then_remove_restores_the_text() {
    let text = r#"{"list":[1,"b",{"c":2}],"other":"x"}"#;
    for index in 0..=3 {
        let mut document = doc(text);
        document.insert(&LIST, index, &Scalar::Int(7)).unwrap();
        assert_eq!(document.remove(&LIST, index).unwrap(), "7");
        assert_eq!(document.to_json(), text);
    }
}

#[test]
fn remove_then_insert_raw_restores_the_text() {
    let text = r#"{"list":[1,"b",{"c":2}],"other":"x"}"#;
    for index in 0..3 {
        let mut document = doc(text);
        let removed = document.remove(&LIST, index).unwrap();
        document.insert_raw(&LIST, index, &removed).unwrap();
        assert_eq!(document.to_json(), text);
    }
}

#[test]
fn indexes_shift_after_insert_and_remove() {
    let mut document = doc(r#"{"list":[10,20,30]}"#);
    let at = |index| [Segment::Key("list"), Segment::Index(index)];
    document.insert(&LIST, 1, &Scalar::Int(15)).unwrap();
    assert_eq!(document.get(&at(1)).unwrap(), Scalar::Int(15));
    assert_eq!(document.get(&at(2)).unwrap(), Scalar::Int(20));
    document.remove(&LIST, 0).unwrap();
    assert_eq!(document.get(&at(0)).unwrap(), Scalar::Int(15));
    assert_eq!(document.get(&at(2)).unwrap(), Scalar::Int(30));
}

#[test]
fn a_path_may_end_in_a_doc_step_over_an_array_document() {
    let mut document = doc(&format!(r#"{{"list":{}}}"#, embed("[1,2]")));
    let path = [Segment::Key("list"), Segment::Doc];
    assert_eq!(document.array_len(&path).unwrap(), 2);
    document.insert(&path, 2, &Scalar::Int(3)).unwrap();
    assert_eq!(document.remove(&path, 0).unwrap(), "1");
    assert_eq!(document.to_json(), format!(r#"{{"list":{}}}"#, embed("[2,3]")));
}

#[test]
fn an_inserted_object_element_accepts_a_later_set() {
    let mut document = doc(r#"{"list":[]}"#);
    document.insert_raw(&LIST, 0, r#"{"a":1,"b":2}"#).unwrap();
    let b = [Segment::Key("list"), Segment::Index(0), Segment::Key("b")];
    document.set(&b, &Scalar::Int(5)).unwrap();
    assert_eq!(document.to_json(), r#"{"list":[{"a":1,"b":5}]}"#);
}

#[test]
fn row_operations_keep_the_text_of_a_sibling_that_a_read_parsed() {
    let text = r#"{"list": [{ "a": 1 }, { "b": 2 }]}"#;
    let first = [Segment::Key("list"), Segment::Index(0), Segment::Key("a")];
    let second = [Segment::Key("list"), Segment::Index(1), Segment::Key("b")];

    let mut document = doc(text);
    assert_eq!(document.get(&first).unwrap(), Scalar::Int(1));
    assert_eq!(document.get(&second).unwrap(), Scalar::Int(2));
    document.insert(&LIST, 2, &Scalar::Int(3)).unwrap();
    assert_eq!(document.to_json(), r#"{"list":[{ "a": 1 },{ "b": 2 },3]}"#);

    let mut document = doc(r#"{"list": [{ "a": 1 }, { "b": 2 }, 3]}"#);
    assert_eq!(document.get(&first).unwrap(), Scalar::Int(1));
    assert_eq!(document.get(&second).unwrap(), Scalar::Int(2));
    assert_eq!(document.remove(&LIST, 2).unwrap(), "3");
    assert_eq!(document.to_json(), r#"{"list":[{ "a": 1 },{ "b": 2 }]}"#);
}

#[test]
fn remove_returns_the_stored_text_of_an_element_that_a_read_parsed() {
    let mut document = doc(r#"{"list": [{ "a": 1 }, 2]}"#);
    let a = [Segment::Key("list"), Segment::Index(0), Segment::Key("a")];
    assert_eq!(document.get(&a).unwrap(), Scalar::Int(1));
    assert_eq!(document.remove(&LIST, 0).unwrap(), r#"{ "a": 1 }"#);
    assert_eq!(document.to_json(), r#"{"list":[2]}"#);
}
