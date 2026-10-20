//! The game interface of `Ff1Document` through `Box<dyn SaveDocument>`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_format::{one_file, sav, slot_name};
use common::ff1_save::{SlotData, character_field, standard_model};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, RowId, SaveDocument, SaveFormat, Value};
use neoffse_format_pr::container::{Container, decode};
use neoffse_format_pr::ff1::{Ff1Format, names};

const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;

fn slot(id: u32, number: u32) -> FieldId {
    character_field(id, names::EQUIPMENT).with_id(number)
}

fn equip(id: u32, number: u32, content: i64) -> Edit {
    Edit::set(slot(id, number), Value::Int(content))
}

fn item(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

fn original() -> String {
    SlotData::standard().render()
}

/// The standard slot opened through the trait, the way the UI holds it.
fn boxed() -> Box<dyn SaveDocument> {
    let format: &dyn SaveFormat = &Ff1Format::new();
    format.open(&one_file(slot_name(1), sav(&original(), &Container::game()))).unwrap()
}

fn text_of(document: &dyn SaveDocument) -> String {
    decode(document.encode().unwrap().get(slot_name(1)).unwrap()).unwrap().json
}

#[test]
fn an_equip_expands_through_the_trait() {
    let edit = equip(1, 1, 104);
    let mut document = boxed();
    let change = document.apply_game(&edit).unwrap();
    let want = standard_model().apply_game(&edit).unwrap();
    assert_eq!(change, want);
    assert_eq!(change.forward.len(), 3, "slot, inventory row, slot count");
}

#[test]
fn the_raw_interface_and_the_game_interface_still_disagree() {
    let remove = Edit::remove(item(89));
    let mut document = boxed();
    let error = document.apply_game(&remove).unwrap_err();
    assert!(matches!(error, EditError::ReadOnly { .. }), "{error:?}");
    assert_eq!(text_of(&*document), original(), "a refused edit changes nothing");
    let mut fresh = boxed();
    fresh.apply(&remove, CapPolicy::Enforce).unwrap();
    assert_ne!(text_of(&*fresh), original());
}

#[test]
fn the_locks_reach_the_trait() {
    let mut document = boxed();
    for key in [62, 89] {
        assert!(document.row_locked(&item(key)).unwrap(), "inventory.{key}");
    }
    assert!(!document.row_locked(&item(100)).unwrap());
    assert!(!document.row_locked(&RowId::new(FieldId::new(names::KEY_ITEMS), 45)).unwrap());
    let slot_count = slot(1, 1).with_name(names::COUNT);
    let placeholder = item(62).field(names::COUNT);
    assert!(document.field_locked(&slot_count).unwrap());
    assert!(document.field_locked(&placeholder).unwrap());
    assert!(!document.field_locked(&FieldId::new(names::GIL)).unwrap());
    let unknown = character_field(1, "nope");
    assert!(matches!(document.field_locked(&unknown), Err(EditError::UnknownField { .. })));
}

#[test]
fn a_game_change_replays_through_apply_all() {
    let mut document = boxed();
    let before = text_of(&*document);
    let change = document.apply_game(&equip(2, 1, 104)).unwrap();
    let equipped = text_of(&*document);
    assert_ne!(equipped, before);
    document.apply_all(&change.inverse, STORAGE_ONLY).unwrap();
    assert_eq!(text_of(&*document), before);
    document.apply_all(&change.forward, STORAGE_ONLY).unwrap();
    assert_eq!(text_of(&*document), equipped);
}
