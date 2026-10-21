//! Property tests of the row operations over arrays of plain values.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::embed;
use neoffse_format_pr::document::{Document, Segment};
use proptest::collection::vec;
use proptest::prelude::*;

const LIST: [Segment<'static>; 1] = [Segment::Key("list")];

/// Compact JSON values: numbers, strings and objects.
fn element() -> impl Strategy<Value = String> {
    prop_oneof![
        any::<i64>().prop_map(|n| n.to_string()),
        "[a-z\\\\\"/ ]{0,6}".prop_map(|s| serde_json::to_string(&s).unwrap()),
        any::<u8>().prop_map(|n| format!(r#"{{"a":{n},"b":[{n},"x"]}}"#)),
    ]
}

fn compact(items: &[String]) -> String {
    format!(r#"{{"before":1.50,"list":[{}],"after":"z"}}"#, items.join(","))
}

fn raw_of(document: &mut Document, index: usize) -> String {
    document.raw(&[Segment::Key("list"), Segment::Index(index)]).unwrap().into_owned()
}

proptest! {
    #[test]
    fn insert_then_remove_is_the_identity(items in vec(element(), 0..6), value in element()) {
        let text = compact(&items);
        for index in 0..=items.len() {
            let mut document = Document::parse(&text).unwrap();
            document.insert_raw(&LIST, index, &value).unwrap();
            prop_assert_eq!(document.remove(&LIST, index).unwrap(), value.clone());
            prop_assert_eq!(document.to_json(), text.clone());
        }
    }

    #[test]
    fn remove_then_insert_raw_is_the_identity(items in vec(element(), 1..6)) {
        let text = compact(&items);
        for (index, item) in items.iter().enumerate() {
            let mut document = Document::parse(&text).unwrap();
            let removed = document.remove(&LIST, index).unwrap();
            prop_assert_eq!(&removed, item);
            document.insert_raw(&LIST, index, &removed).unwrap();
            prop_assert_eq!(document.to_json(), text.clone());
        }
    }

    #[test]
    fn a_sequence_of_row_edits_matches_a_vector_model(
        start in vec(element(), 0..4),
        ops in vec((any::<bool>(), any::<prop::sample::Index>(), element()), 0..24),
    ) {
        let mut model = start.clone();
        let mut document = Document::parse(&compact(&start)).unwrap();
        for (insert, pick, value) in ops {
            if insert {
                let index = pick.index(model.len() + 1);
                document.insert_raw(&LIST, index, &value).unwrap();
                model.insert(index, value);
            } else if !model.is_empty() {
                let index = pick.index(model.len());
                prop_assert_eq!(document.remove(&LIST, index).unwrap(), model.remove(index));
            }
            prop_assert_eq!(document.array_len(&LIST).unwrap(), model.len());
        }
        for (index, expected) in model.iter().enumerate() {
            prop_assert_eq!(&raw_of(&mut document, index), expected);
        }
        prop_assert_eq!(document.to_json(), compact(&model));
    }

    #[test]
    fn no_row_operation_panics(index in any::<usize>(), pick in 0usize..6, json in ".{0,8}") {
        let inner = embed(r#"{"target":[1,2]}"#);
        let text = format!(r#"{{"list":[1,2],"o":{{"a":1}},"s":{inner},"n":5}}"#);
        let mut document = Document::parse(&text).unwrap();
        let paths: [Vec<Segment<'_>>; 6] = [
            vec![Segment::Key("list")],
            vec![Segment::Key("o")],
            vec![Segment::Key("s"), Segment::Doc, Segment::Key("target")],
            vec![Segment::Key("s")],
            vec![Segment::Key("n"), Segment::Doc],
            vec![],
        ];
        let path = &paths[pick];
        let _ = document.array_len(path);
        let _ = document.insert_raw(path, index, &json);
        let _ = document.remove(path, index);
        let _ = document.to_json();
    }
}
