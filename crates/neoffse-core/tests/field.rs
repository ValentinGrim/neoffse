//! Field ids, row ids and values.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::collections::HashSet;

use neoffse_core::{FieldId, Key, RowId, Value, ValueKind};

#[test]
fn new_holds_the_section_alone() {
    let id = FieldId::new("gil");
    assert_eq!(id.section(), "gil");
    assert_eq!(id.keys(), [Key::Name("gil")]);
    assert_eq!(id.to_string(), "gil");
    assert_eq!(id.parent(), None);
}

#[test]
fn builders_append_in_order() {
    let id = FieldId::new("character").with_id(1).with_name("max_hp");
    assert_eq!(id.keys(), [Key::Name("character"), Key::Id(1), Key::Name("max_hp")]);
    assert_eq!(id.to_string(), "character.1.max_hp");
}

#[test]
fn ids_render_as_decimal_numbers() {
    assert_eq!(FieldId::new("a").with_id(0).to_string(), "a.0");
    assert_eq!(FieldId::new("a").with_id(4_294_967_295).to_string(), "a.4294967295");
    assert_eq!(FieldId::new("a").with_id(u32::MAX).to_string(), "a.4294967295");
}

#[test]
fn a_name_and_an_id_with_the_same_digits_differ() {
    let name = FieldId::new("a").with_name("1");
    let id = FieldId::new("a").with_id(1);
    assert_eq!(name.to_string(), id.to_string());
    assert_ne!(name, id);
    let set: HashSet<FieldId> = [name, id].into_iter().collect();
    assert_eq!(set.len(), 2);
}

#[test]
fn equality_and_hash_follow_the_keys() {
    let a = FieldId::new("a").with_id(1);
    let b = FieldId::new("a").with_id(1);
    assert_eq!(a, b);
    let set: HashSet<FieldId> = [a, b].into_iter().collect();
    assert_eq!(set.len(), 1);
    let one_then_name = FieldId::new("a").with_id(1).with_name("b");
    let name_then_one = FieldId::new("a").with_name("b").with_id(1);
    assert_ne!(one_then_name, name_then_one);
}

#[test]
fn starts_with_is_a_key_prefix() {
    let hp = FieldId::new("character").with_id(1).with_name("hp");
    assert!(hp.starts_with(&FieldId::new("character")));
    assert!(hp.starts_with(&FieldId::new("character").with_id(1)));
    assert!(hp.starts_with(&hp));
    assert!(!hp.starts_with(&FieldId::new("character").with_id(2)));
    assert!(!hp.starts_with(&hp.clone().with_name("x")));
    assert!(!hp.starts_with(&FieldId::new("char")));
}

#[test]
fn a_derived_id_leaves_its_parent_unchanged() {
    let parent = FieldId::new("character").with_id(1);
    let earlier = parent.clone();
    let child = parent.clone().with_name("hp");
    assert_eq!(parent, earlier);
    assert_ne!(child, parent);
}

#[test]
fn parent_drops_the_last_key() {
    let hp = FieldId::new("character").with_id(1).with_name("hp");
    let character = hp.parent().unwrap();
    assert_eq!(character, FieldId::new("character").with_id(1));
    let section = character.parent().unwrap();
    assert_eq!(section, FieldId::new("character"));
    assert_eq!(section.parent(), None);
}

#[test]
fn ids_sort_in_a_stable_total_order() {
    let ids = [
        FieldId::new("gil"),
        FieldId::new("character").with_id(2).with_name("hp"),
        FieldId::new("character").with_id(1).with_name("hp"),
        FieldId::new("character").with_id(1),
        FieldId::new("character").with_name("1"),
        FieldId::new("inventory").with_id(7).with_name("count"),
        FieldId::new("character"),
        FieldId::new("inventory").with_id(3),
    ];
    let mut forward = ids.to_vec();
    let mut backward: Vec<FieldId> = ids.iter().rev().cloned().collect();
    forward.sort();
    backward.sort();
    assert_eq!(forward, backward);
    assert_eq!(forward.len(), 8);
}

#[test]
fn a_row_id_carries_the_collection_and_the_key() {
    let row = RowId::new(FieldId::new("inventory"), 7);
    assert_eq!(row.key(), 7);
    assert_eq!(row.collection(), FieldId::new("inventory"));
    assert_eq!(row.id().to_string(), "inventory.7");
    assert_eq!(row.to_string(), "inventory.7");
}

#[test]
fn a_row_field_extends_the_row_id() {
    let row = RowId::new(FieldId::new("inventory"), 7);
    let count = row.field("count");
    assert_eq!(count.to_string(), "inventory.7.count");
    assert!(count.starts_with(row.id()));
    assert_eq!(count.parent().as_ref(), Some(row.id()));
}

#[test]
fn rows_differ_by_collection_and_by_key() {
    let a = RowId::new(FieldId::new("inventory"), 7);
    let b = RowId::new(FieldId::new("inventory"), 8);
    let c = RowId::new(FieldId::new("bag"), 7);
    assert_ne!(a, b);
    assert_ne!(a, c);
    assert_ne!(b, c);
    let nested = RowId::new(FieldId::new("character").with_id(1).with_name("spells"), 5);
    assert_eq!(nested.collection(), FieldId::new("character").with_id(1).with_name("spells"));
    assert_eq!(nested.key(), 5);
    assert_eq!(nested.to_string(), "character.1.spells.5");
}

#[test]
fn values_report_their_kind() {
    assert_eq!(Value::Int(0).kind(), ValueKind::Int);
    assert_eq!(Value::Bool(false).kind(), ValueKind::Bool);
    assert_eq!(Value::Text(String::new()).kind(), ValueKind::Text);
    assert_eq!(ValueKind::Int.to_string(), "integer");
    assert_eq!(ValueKind::Bool.to_string(), "boolean");
    assert_eq!(ValueKind::Text.to_string(), "text");
}

#[test]
fn values_of_different_kinds_never_compare_equal() {
    assert_ne!(Value::Int(0), Value::Bool(false));
    assert_ne!(Value::Int(0), Value::Text("0".into()));
    assert_ne!(Value::Text(String::new()), Value::Text(" ".into()));
}
