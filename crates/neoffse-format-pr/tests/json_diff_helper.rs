//! The test-only structural diff of `common::json_diff`: each kind of change on the
//! synthetic slot and on small hand-written texts.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::SlotData;
use common::json_diff::{Change, Kind, added, assert_changes, diff, layout, layout_chain, removed, reordered, value};
use common::json_tree::Tree;
use common::{character_path, embed};
use neoffse_format_pr::document::{Document, Scalar, Segment};

const LEVEL: &str = "userData.ownedCharacterList.target[1].parameter.addtionalLevel";
const LIST: [Segment<'static>; 5] = [
    Segment::Key("userData"),
    Segment::Doc,
    Segment::Key("normalOwnedItemList"),
    Segment::Doc,
    Segment::Key("target"),
];

fn standard() -> String {
    SlotData::standard().render()
}

/// The standard slot after `edit`, which must change the text.
fn edited(edit: impl FnOnce(&mut Document)) -> String {
    let before = standard();
    let mut document = Document::parse(&before).unwrap();
    edit(&mut document);
    let after = document.to_json();
    assert_ne!(after, before);
    after
}

/// `before` with the first `from` replaced by `to`, which must change the text.
fn replaced(before: &str, from: &str, to: &str) -> String {
    let after = before.replacen(from, to, 1);
    assert_ne!(after, before, "{from} not found");
    after
}

#[test]
fn identical_texts_have_no_change() {
    let text = standard();
    assert_eq!(diff(&text, &text), []);
    assert_eq!(diff("{}", "{}"), []);
}

#[test]
fn one_digit_in_a_nested_document_is_one_value_change() {
    let before = standard();
    let after = replaced(&before, "owendGil\\\":123456", "owendGil\\\":123457");
    assert_eq!(diff(&before, &after), [value("userData.owendGil")]);
    let path = character_path(1, &[Segment::Doc, Segment::Key("parameter"), Segment::Doc, Segment::Key("addtionalLevel")]);
    let after = edited(|document| document.set(&path, &Scalar::Int(77)).unwrap());
    assert_eq!(diff(&before, &after), [value(LEVEL)]);
}

#[test]
fn an_extra_changed_key_is_reported() {
    let after = edited(|document| {
        document
            .set(&[Segment::Key("userData"), Segment::Doc, Segment::Key("owendGil")], &Scalar::Int(5))
            .unwrap();
        document.set(&[Segment::Key("playTime")], &Scalar::Int(5)).unwrap();
    });
    assert_eq!(diff(&standard(), &after), [value("playTime"), value("userData.owendGil")]);
}

#[test]
fn a_reordered_key_is_reported() {
    let before = standard();
    let after = replaced(&before, "\"clearFlag\":1,\"isCompleteFlag\":1", "\"isCompleteFlag\":1,\"clearFlag\":1");
    assert_eq!(diff(&before, &after), [reordered("")]);
    let small = r#"{"d":"{\"a\":1,\"b\":2}"}"#;
    assert_eq!(diff(small, r#"{"d":"{\"b\":2,\"a\":1}"}"#), [reordered("d")]);
}

#[test]
fn a_float_rewrite_is_reported() {
    let before = standard();
    let after = replaced(&before, "13621.814333377138", "13621.814333377139");
    assert_eq!(diff(&before, &after), [value("userData.playTime")]);
    assert_eq!(diff(r#"{"x":1.0}"#, r#"{"x":1}"#), [value("x")]);
}

#[test]
fn a_dropped_key_and_an_added_key_are_reported() {
    let before = standard();
    assert_eq!(diff(&before, &replaced(&before, "\"clearFlag\":1,", "")), [removed("clearFlag")]);
    assert_eq!(diff(&before, &replaced(&before, "{\"id\":1,", "{\"extra\":7,\"id\":1,")), [added("extra")]);
}

#[test]
fn whitespace_is_a_layout_change() {
    let spaced = SlotData {
        spaced: true,
        ..SlotData::standard()
    };
    assert_eq!(diff(&standard(), &spaced.render()), [layout("")]);
}

#[test]
fn a_reescaped_string_is_a_value_change() {
    let path = character_path(3, &[Segment::Doc, Segment::Key("name")]);
    let after = edited(|document| document.set_raw(&path, "\"Zo\\u00e9\"").unwrap());
    assert_eq!(diff(&standard(), &after), [value("userData.ownedCharacterList.target[3].name")]);
}

#[test]
fn a_reescaped_document_token_is_a_layout_change() {
    assert_eq!(diff(r#"{"d":"{\"a\":1}"}"#, r#"{"d":"{\u0022a\u0022:1}"}"#), [layout("d")]);
}

#[test]
fn an_array_insert_is_one_added_element() {
    let before = r#"{"t":[1,2,3,4]}"#;
    assert_eq!(diff(before, r#"{"t":[1,2,9,3,4]}"#), [added("t[2]")]);
    assert_eq!(diff(before, r#"{"t":[9,1,2,3,4]}"#), [added("t[0]")]);
    assert_eq!(diff(before, r#"{"t":[1,2,3,4,9]}"#), [added("t[4]")]);
    assert_eq!(diff(r#"{"t":[1,2,9,3,4]}"#, before), [removed("t[2]")]);
    assert_eq!(diff(r#"{"t":[9,1,2,3,4]}"#, before), [removed("t[0]")]);
    assert_eq!(diff(r#"{"t":[1,2,3,4,9]}"#, before), [removed("t[4]")]);
    assert_eq!(diff(before, r#"{"t":[1,3,4]}"#), [removed("t[1]")]);

    let before = standard();
    let item = embed(r#"{"contentId":9,"count":1}"#);
    for index in [0, 7, 29] {
        let after = edited(|document| document.insert_raw(&LIST, index, &item).unwrap());
        assert_eq!(diff(&before, &after), [added(&format!("userData.normalOwnedItemList.target[{index}]"))]);
    }
    let after = edited(|document| {
        document.remove(&LIST, 5).unwrap();
    });
    assert_eq!(diff(&before, &after), [removed("userData.normalOwnedItemList.target[5]")]);
}

#[test]
fn an_unequal_middle_pairs_from_the_start() {
    assert_eq!(diff(r#"{"t":[1,2,3]}"#, r#"{"t":[1,9,8,3]}"#), [value("t[1]"), added("t[2]")]);
    assert_eq!(diff(r#"{"t":[1,9,8,3]}"#, r#"{"t":[1,2,3]}"#), [value("t[1]"), removed("t[2]")]);
}

#[test]
fn an_empty_string_is_not_an_empty_object() {
    assert_eq!(diff(r#"{"d":""}"#, r#"{"d":"{}"}"#), [value("d")]);
    assert_eq!(diff(r#"{"d":"x"}"#, r#"{"d":"{}"}"#), [value("d")]);
}

#[test]
fn a_change_of_shape_is_a_value_change() {
    assert_eq!(diff(r#"{"x":1}"#, r#"{"x":{"a":1}}"#), [value("x")]);
    assert_eq!(diff(r#"{"x":{"a":1}}"#, r#"{"x":[1]}"#), [value("x")]);
    assert_eq!(diff(r#"{"a":1}"#, "[1]"), [value("")]);
}

#[test]
fn the_diff_is_symmetric() {
    let before = standard();
    let pairs = [
        (before.clone(), replaced(&before, "\"clearFlag\":1,", "")),
        (
            before.clone(),
            edited(|document| document.insert_raw(&LIST, 3, &embed(r#"{"contentId":9,"count":1}"#)).unwrap()),
        ),
        (r#"{"t":[1,2,3]}"#.to_owned(), r#"{"t":[1,9,8,3],"u":1}"#.to_owned()),
    ];
    for (left, right) in pairs {
        let forward = diff(&left, &right);
        assert_ne!(forward.len(), 0);
        let mut mirrored: Vec<Change> = forward
            .iter()
            .map(|item| Change {
                kind: match item.kind {
                    Kind::Added => Kind::Removed,
                    Kind::Removed => Kind::Added,
                    other => other,
                },
                path: item.path.clone(),
            })
            .collect();
        mirrored.sort();
        assert_eq!(diff(&right, &left), mirrored);
    }
}

#[test]
fn layout_chain_lists_the_spaced_ancestors() {
    let compact = Tree::parse(&standard());
    let spaced = Tree::parse(&compact.spaced(4));
    let level = vec![value("userData.ownedCharacterList.target[0].parameter.addtionalLevel")];
    assert_eq!(
        layout_chain(&spaced, &level),
        [
            layout(""),
            layout("userData"),
            layout("userData.ownedCharacterList"),
            layout("userData.ownedCharacterList.target")
        ]
    );
    assert_eq!(layout_chain(&compact, &level), []);
    assert_eq!(layout_chain(&spaced, &[value("userData.owendGil")]), [layout(""), layout("userData")]);
    assert_eq!(layout_chain(&spaced, &[value("playTime")]), [layout("")]);
    assert_eq!(layout_chain(&spaced, &[value("")]), []);
    let rows = [
        added("userData.normalOwnedItemList.target[29]"),
        removed("userData.normalOwnedItemList.target[5]"),
    ];
    assert_eq!(layout_chain(&spaced, &rows).len(), 4);
}

#[test]
fn diff_against_the_spaced_text_is_all_layout() {
    let compact = standard();
    let spaced = Tree::parse(&compact).spaced(4);
    let changes = diff(&compact, &spaced);
    assert!(changes.iter().all(|item| item.kind == Kind::Layout), "{changes:?}");
    assert!(changes.contains(&layout("")));
}

#[test]
#[should_panic(expected = "unexpected change")]
fn a_list_naming_one_of_two_edits_fails() {
    assert_changes("two edits", &[value("a"), value("b")], &[value("a")]);
}

#[test]
#[should_panic(expected = "missing change")]
fn a_list_naming_an_edit_that_did_not_happen_fails() {
    assert_changes("one edit", &[value("a")], &[value("a"), value("b")]);
}

#[test]
fn assert_changes_ignores_order_and_names_both_directions() {
    assert_changes("same", &[value("b"), added("a")], &[added("a"), value("b")]);
    let panic = std::panic::catch_unwind(|| assert_changes("both", &[value("x")], &[added("y")])).unwrap_err();
    let message = panic.downcast_ref::<String>().unwrap();
    assert!(message.contains("both: unexpected change: value x"), "{message}");
    assert!(message.contains("both: missing change: added y"), "{message}");
    assert_eq!(added("").to_string(), "added <root>");
}
