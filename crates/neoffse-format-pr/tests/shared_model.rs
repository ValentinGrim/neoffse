//! `SlotModel` through `Ff1Model`: the constants, the accessors and the
//! edit methods reach the inherent methods of the model.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{SlotData, character_field};
use neoffse_core::{CapPolicy, Capability, Edit, FieldId, Game, RowId, Value};
use neoffse_data::tables_for;
use neoffse_format_pr::document::Document;
use neoffse_format_pr::ff1::{Ff1Model, names};
use neoffse_format_pr::shared::SlotModel;

/// A model of the standard slot, built through the trait alone.
fn probe<M: SlotModel>() -> (M, String) {
    let text = SlotData::standard().render();
    let tables = tables_for(M::GAME).unwrap();
    (M::new(Document::parse(&text).unwrap(), tables), text)
}

fn item(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

#[test]
fn the_constants_are_those_of_ff1() {
    assert_eq!(<Ff1Model as SlotModel>::GAME, Game::Ff1Pr);
    assert_eq!(
        <Ff1Model as SlotModel>::CAPABILITIES,
        [Capability::Party, Capability::Inventory, Capability::Magic]
    );
    assert_eq!(<Ff1Model as SlotModel>::DOCUMENT, "Ff1Document");
}

#[test]
fn the_accessors_reach_the_document_and_the_tables() {
    let (model, text) = probe::<Ff1Model>();
    assert!(std::ptr::eq(model.tables(), tables_for(Game::Ff1Pr).unwrap()));
    assert_eq!(SlotModel::document(&model).to_json(), text);
}

#[test]
fn the_reads_reach_the_model() {
    let (mut model, _) = probe::<Ff1Model>();
    assert_eq!(SlotModel::rows(&mut model, &FieldId::new(names::CHARACTER)).unwrap(), [1, 2, 3, 4]);
    assert_eq!(SlotModel::get(&mut model, &FieldId::new(names::GIL)).unwrap(), Value::Int(123_456));
    assert_eq!(SlotModel::field_meta(&model, &FieldId::new(names::GIL)).unwrap().label, "Gil");
    assert!(SlotModel::collection_meta(&model, &FieldId::new(names::INVENTORY)).is_some());
    assert!(SlotModel::collection_meta(&model, &FieldId::new(names::GIL)).is_none());
}

#[test]
fn apply_game_expands_an_equip_and_apply_does_not() {
    let (mut model, _) = probe::<Ff1Model>();
    // Content 92 is worn by character 2: the game interface moves it, the raw one sets the slot only.
    let edit = Edit::set(character_field(1, names::EQUIPMENT).with_id(1), Value::Int(92));
    let change = SlotModel::apply_game(&mut model, &edit).unwrap();
    assert!(change.forward.len() > 1, "{:?}", change.forward);

    let (mut raw, _) = probe::<Ff1Model>();
    let inverse = SlotModel::apply(&mut raw, &edit, CapPolicy::Enforce).unwrap();
    assert!(matches!(inverse, Edit::Set { .. }), "{inverse:?}");
}

#[test]
fn the_locks_reach_the_model() {
    let (mut model, _) = probe::<Ff1Model>();
    let slot_count = character_field(1, names::EQUIPMENT).with_id(1).with_name(names::COUNT);
    assert!(SlotModel::field_locked(&model, &slot_count).unwrap());
    assert!(!SlotModel::field_locked(&model, &FieldId::new(names::GIL)).unwrap());
    assert!(SlotModel::row_locked(&mut model, &item(62)).unwrap(), "placeholder");
    assert!(!SlotModel::row_locked(&mut model, &item(17)).unwrap());
}

#[test]
fn the_summary_and_the_view_reach_the_model() {
    let (mut model, _) = probe::<Ff1Model>();
    assert_eq!(SlotModel::summary(&mut model).unwrap().party.len(), 4);
    let view = SlotModel::view(&mut model, neoffse_data::Language::En).unwrap();
    let found: Vec<Capability> = view.sections.iter().map(|section| section.capability).collect();
    assert_eq!(found, <Ff1Model as SlotModel>::CAPABILITIES);
}
