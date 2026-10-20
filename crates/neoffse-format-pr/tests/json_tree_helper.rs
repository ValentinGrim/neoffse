//! The test-only JSON tree of `common::json_tree`: paths, lookups, whitespace and re-rendering.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::SlotData;
use common::json_tree::{Shape, Tree};

const INVENTORY: &str = "userData.normalOwnedItemList.target";
const FIRST: &str = "userData.ownedCharacterList.target[0]";

fn standard() -> Tree {
    Tree::parse(&SlotData::standard().render())
}

#[test]
fn paths_step_through_documents() {
    let tree = standard();
    assert_eq!(tree.raw_at("userData.owendGil"), Some("123456"));
    assert_eq!(tree.raw_at("userData.ownedCharacterList.target[0].name"), Some("\"Aria\""));
    assert_eq!(tree.raw_at("userData.ownedCharacterList.target[3].name"), Some("\"Zoé\""));
    assert_eq!(tree.int("userData.normalOwnedItemList.target[11].contentId"), Some(89));
    assert_eq!(tree.at("").map(Tree::raw), Some(tree.raw()));
    assert_eq!(tree.array_len(INVENTORY), Some(29));
    assert_eq!(tree.array_len("userData.importantOwendItemList.target"), Some(9));
}

#[test]
fn a_document_opens_at_the_last_step_too() {
    let tree = standard();
    let user_data = tree.at("userData").unwrap();
    assert!(user_data.raw().starts_with("{\"corpsList\""));
    assert!(tree.raw_at("userData.configData").is_some_and(|raw| raw == "\"\""));
}

#[test]
fn unknown_steps_are_none() {
    let tree = standard();
    assert!(tree.at("userData.noSuchKey").is_none());
    assert!(tree.at("userData.normalOwnedItemList.target[29]").is_none());
    assert!(tree.at("userData.owendGil.deeper").is_none());
    assert!(tree.at("userData.owendGil[0]").is_none());
    assert!(tree.at("userData.normalOwnedItemList.target[x]").is_none());
    assert!(tree.at("userData.normalOwnedItemList.target[1").is_none());
    assert!(tree.at("userData..owendGil").is_none());
    assert!(tree.at("id[0]").is_none());
    assert!(tree.int("userData.noSuchKey").is_none());
    assert!(tree.array_len("userData.owendGil").is_none());
}

#[test]
fn find_helpers_return_the_first_match() {
    let tree = standard();
    assert_eq!(tree.find_row(INVENTORY, "contentId", 89), Some(11));
    assert_eq!(tree.find_row(INVENTORY, "contentId", 9999), None);
    assert_eq!(tree.find_row(INVENTORY, "count", 9), Some(1));
    assert_eq!(tree.find_row("userData.owendGil", "contentId", 89), None);
    let keys = format!("{FIRST}.equipmentList.keys");
    assert_eq!(tree.find_int(&keys, 1), Some(0));
    assert_eq!(tree.find_int(&keys, 5), Some(4));
    assert_eq!(tree.find_int(&keys, 6), None);
}

#[test]
fn int_refuses_a_float() {
    let tree = standard();
    assert_eq!(tree.int("userData.playTime"), None);
    assert_eq!(tree.raw_at("userData.playTime"), Some("13621.814333377138"));
    assert_eq!(Tree::parse("[4294967295,-3]").int("[0]"), Some(4_294_967_295));
    assert_eq!(Tree::parse("[4294967295,-3]").int("[1]"), Some(-3));
}

#[test]
fn a_string_that_is_not_json_stays_a_scalar() {
    let mut slot = SlotData::standard();
    slot.characters[0].name = "{oops".to_owned();
    slot.characters[1].name = "[1]".to_owned();
    let tree = Tree::parse(&slot.render());
    let name_shape = |index: usize| match tree.at(&format!("userData.ownedCharacterList.target[{index}]")).unwrap().shape() {
        Shape::Object(members) => match members.iter().find(|(key, _)| key == "name").unwrap().1.shape() {
            Shape::Scalar => "scalar",
            Shape::Doc(_) => "doc",
            Shape::Object(_) | Shape::Array(_) => "container",
        },
        _ => "not an object",
    };
    assert_eq!(name_shape(0), "scalar");
    // A name of `[1]` is valid JSON, so rule D opens it. The game has no such name.
    assert_eq!(name_shape(1), "doc");
    assert_eq!(tree.raw_at("userData.ownedCharacterList.target[0].name"), Some("\"{oops\""));
    assert_eq!(tree.raw_at("userData.ownedCharacterList.target[1].name"), Some("[1]"));
}

#[test]
fn an_empty_string_and_a_padded_document_are_scalars() {
    let tree = Tree::parse(r#"{"empty":"","padded":"{} ","lead":" {}","open":"{"}"#);
    for key in ["empty", "padded", "lead", "open"] {
        let Shape::Object(members) = tree.shape() else { panic!("object") };
        assert!(
            matches!(members.iter().find(|(name, _)| name == key).unwrap().1.shape(), Shape::Scalar),
            "{key}"
        );
    }
}

#[test]
fn own_whitespace_follows_the_text() {
    let compact = standard();
    assert_eq!(compact.own_ws(), "");
    let spaced_slot = SlotData {
        spaced: true,
        ..SlotData::standard()
    };
    let spaced = Tree::parse(&spaced_slot.render());
    assert_ne!(spaced.own_ws(), "");
    assert_eq!(spaced.at("userData").unwrap().own_ws(), "");
    // Inner runs first, then the whitespace around the root.
    let padded = Tree::parse(" [ 1 ,\n2]\t");
    assert_eq!(padded.own_ws(), "  \n \t");
}

#[test]
fn spaced_renders_a_parsable_text_and_is_idempotent() {
    let tree = standard();
    let text = tree.spaced(4);
    assert_ne!(text, tree.raw());
    assert_eq!(Tree::parse(&text).spaced(4), text);
    assert_eq!(tree.spaced(0), tree.raw());
    let spaced = Tree::parse(&text);
    assert_ne!(spaced.own_ws(), "");
    assert_ne!(spaced.at("userData").unwrap().own_ws(), "");
    assert_ne!(spaced.at("userData.ownedCharacterList").unwrap().own_ws(), "");
    assert_ne!(spaced.at("userData.ownedCharacterList.target").unwrap().own_ws(), "");
    assert_eq!(spaced.at(FIRST).unwrap().own_ws(), "");
    assert_eq!(spaced.at(&format!("{INVENTORY}[0]")).unwrap().own_ws(), "");
    assert_eq!(spaced.raw_at("userData.playTime"), Some("13621.814333377138"));
}

#[test]
#[should_panic(expected = "repeated key")]
fn a_repeated_key_panics() {
    Tree::parse(r#"{"a":1,"b":{"c":2,"c":3}}"#);
}

#[test]
#[should_panic(expected = "repeated key")]
fn a_repeated_key_inside_a_document_panics() {
    Tree::parse(r#"{"a":"{\"c\":2,\"c\":3}"}"#);
}

#[test]
#[should_panic(expected = "invalid JSON")]
fn invalid_json_panics() {
    Tree::parse(r#"{"a":1,}"#);
}

#[test]
#[should_panic(expected = "invalid JSON")]
fn two_values_panic() {
    Tree::parse("{} {}");
}
