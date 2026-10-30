//! `Ff1Model::view` against the synthetic slot.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{INVENTORY, KEY_ITEMS, Row, SlotData, model};
use neoffse_core::{CapPolicy, Capability, Edit, EditError, FieldId, FormatError, RowId, SaveView, Value};
use neoffse_data::Language;
use neoffse_format_pr::ff1::{CAPABILITIES, Ff1Model, names};

const NAMES: [&str; 4] = ["Aria", "Bram", "Cleo", "Zoé"];

fn view_of(slot: &SlotData) -> (Ff1Model, SaveView) {
    let mut model = model(&slot.render());
    let view = model.view(Language::En).unwrap();
    (model, view)
}

fn character(id: u32) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id)
}

fn item(list: &'static str, key: u32) -> FieldId {
    FieldId::new(list).with_id(key)
}

#[test]
fn sections_follow_the_capabilities() {
    let (_, view) = view_of(&SlotData::standard());
    let found: Vec<Capability> = view.sections.iter().map(|section| section.capability).collect();
    assert_eq!(found, CAPABILITIES);
}

#[test]
fn party_lists_the_characters_in_document_order() {
    let (mut model, view) = view_of(&SlotData::standard());
    let party = view.section(Capability::Party).unwrap();
    assert_eq!(party.entities.len(), 4);
    for (index, entity) in party.entities.iter().enumerate() {
        let id = u32::try_from(index).unwrap() + 1;
        assert_eq!(entity.id, character(id));
        assert_eq!(entity.label, NAMES[index]);
        let expected = names::party_fields(id);
        let found: Vec<&FieldId> = entity.fields.iter().map(|field| &field.id).collect();
        assert_eq!(found, expected.iter().collect::<Vec<_>>());
        for field in &entity.fields {
            assert_eq!(Some(field.meta.clone()), model.field_meta(&field.id), "{}", field.id);
            assert_eq!(model.get(&field.id).unwrap(), field.value, "{}", field.id);
        }
    }
}

#[test]
fn document_order_wins_over_id_order() {
    let mut slot = SlotData::standard();
    slot.characters.reverse();
    let (_, view) = view_of(&slot);
    let ids: Vec<FieldId> = view.section(Capability::Party).unwrap().entities.iter().map(|e| e.id.clone()).collect();
    assert_eq!(ids, [character(4), character(3), character(2), character(1)]);
}

#[test]
fn an_empty_name_gets_a_generic_label() {
    let mut slot = SlotData::standard();
    slot.characters[1].name = String::new();
    let (_, view) = view_of(&slot);
    let party = view.section(Capability::Party).unwrap();
    assert_eq!(party.entities[1].label, "Character 2");
    assert_eq!(party.entities[0].label, "Aria");
}

#[test]
fn inventory_lists_gil_then_rows_then_key_items() {
    let (_, view) = view_of(&SlotData::standard());
    let entities = &view.section(Capability::Inventory).unwrap().entities;
    assert_eq!(entities.len(), 1 + INVENTORY.len() + KEY_ITEMS.len());
    let gil = &entities[0];
    assert_eq!(gil.id, FieldId::new(names::GIL));
    assert_eq!(gil.label, "Gil");
    assert_eq!(gil.fields.len(), 1);
    assert_eq!(gil.fields[0].value, Value::Int(123_456));
    let rows = INVENTORY
        .iter()
        .map(|(id, count)| (names::INVENTORY, *id, i64::from(*count)))
        .chain(KEY_ITEMS.iter().map(|id| (names::KEY_ITEMS, *id, 1)));
    for (entity, (list, key, count)) in entities[1..].iter().zip(rows) {
        assert_eq!(entity.id, item(list, key));
        assert_eq!(entity.fields.len(), 1);
        assert_eq!(entity.fields[0].id, item(list, key).with_name(names::COUNT));
        assert_eq!(entity.fields[0].value, Value::Int(count));
    }
}

#[test]
fn item_labels_follow_the_language() {
    let tables = neoffse_data::tables_for(neoffse_format_pr::ff1::GAME).unwrap();
    let key = INVENTORY
        .iter()
        .map(|(id, _)| *id)
        .find(|id| tables.content(*id).is_some_and(|entry| entry.name_en != entry.name_fr))
        .expect("an inventory id with two different names");
    let entry = tables.content(key).unwrap();
    let mut model = model(&SlotData::standard().render());
    for language in Language::ALL {
        let view = model.view(*language).unwrap();
        let label = &view.find(&item(names::INVENTORY, key).with_name(names::COUNT)).unwrap().0.label;
        assert_eq!(Some(label.as_str()), entry.name(*language));
    }
    assert_ne!(entry.name(Language::En), entry.name(Language::Fr));
}

#[test]
fn an_unknown_id_falls_back_to_the_id() {
    let mut slot = SlotData::standard();
    slot.inventory.push(Row::new(999, 1));
    slot.key_items.push(Row::new(999, 1));
    let (_, view) = view_of(&slot);
    let label = |list, key| view.find(&item(list, key).with_name(names::COUNT)).unwrap().0.label.clone();
    assert_eq!(label(names::INVENTORY, 999), "Item 999");
    assert_eq!(label(names::KEY_ITEMS, 999), "Key item 999");
}

#[test]
fn collections_sit_in_their_sections() {
    let (model, view) = view_of(&SlotData::standard());
    let party = view.section(Capability::Party).unwrap();
    let character = FieldId::new(names::CHARACTER);
    assert_eq!(party.collections, [model.collection_meta(&character).unwrap()]);
    assert!(!party.collections[0].insertable && !party.collections[0].removable);
    let inventory = view.section(Capability::Inventory).unwrap();
    let lists = [FieldId::new(names::INVENTORY), FieldId::new(names::KEY_ITEMS)];
    let expected: Vec<_> = lists.iter().map(|list| model.collection_meta(list).unwrap()).collect();
    assert_eq!(inventory.collections, expected);
    for (list, meta) in lists.iter().zip(&expected) {
        assert_eq!(view.collection(list), Some(meta));
    }
    assert_eq!(view.collection(&character), Some(&party.collections[0]));
}

#[test]
fn an_absent_key_leaves_the_field_out() {
    let mut slot = SlotData::standard();
    slot.characters[2].equipment[2].count = None;
    slot.characters[0].charges.retain(|(level, _)| *level != 3);
    let (_, view) = view_of(&slot);
    let party = &view.section(Capability::Party).unwrap().entities;
    let count_3 = character(3).with_name(names::EQUIPMENT).with_id(3).with_name(names::COUNT);
    assert_eq!(party[2].fields.len(), 42);
    assert!(party[2].fields.iter().all(|field| field.id != count_3));
    let charges_3 = character(1).with_name(names::CHARGES).with_id(3);
    assert_eq!(party[0].fields.len(), 42);
    assert!(party[0].fields.iter().all(|field| field.id != charges_3));
    assert_eq!(party[1].fields.len(), 43);
    assert_eq!(party[3].fields.len(), 43);

    let slot = SlotData {
        gil: None,
        ..SlotData::standard()
    };
    let (_, view) = view_of(&slot);
    let entities = &view.section(Capability::Inventory).unwrap().entities;
    assert_eq!(entities.len(), INVENTORY.len() + KEY_ITEMS.len());
    assert!(entities.iter().all(|entity| entity.id != FieldId::new(names::GIL)));
}

#[test]
fn another_failure_is_a_decode_error() {
    let mut slot = SlotData::standard();
    slot.characters[0].level = "69.5".to_owned();
    let mut model = model(&slot.render());
    let Err(FormatError::Decode { source }) = model.view(Language::En) else {
        panic!("a float level must fail the view")
    };
    assert!(matches!(source.downcast_ref::<EditError>(), Some(EditError::Format { .. })));
}

#[test]
fn the_view_is_a_snapshot() {
    let (mut model, view) = view_of(&SlotData::standard());
    let kept = view.clone();
    let edit = Edit::set(FieldId::new(names::GIL), Value::Int(5));
    model.apply(&edit, CapPolicy::Enforce).unwrap();
    let fresh = model.view(Language::En).unwrap();
    assert_ne!(fresh, kept);
    assert_eq!(kept, view);
}

#[test]
fn an_insert_shows_at_its_position() {
    let (mut model, _) = view_of(&SlotData::standard());
    let row = RowId::new(FieldId::new(names::INVENTORY), 4);
    let edit = Edit::insert(row.clone(), Some(3), vec![(names::COUNT, Value::Int(1))]);
    model.apply(&edit, CapPolicy::Enforce).unwrap();
    let view = model.view(Language::En).unwrap();
    let entities = &view.section(Capability::Inventory).unwrap().entities;
    assert_eq!(entities[1 + 3].id, *row.id());
    assert_eq!(entities.len(), 1 + INVENTORY.len() + 1 + KEY_ITEMS.len());
}

#[test]
fn view_changes_nothing() {
    let slot = SlotData {
        spaced: true,
        ..SlotData::standard()
    };
    let mut model = model(&slot.render());
    let before = model.document().to_json();
    model.view(Language::Fr).unwrap();
    assert_eq!(model.document().to_json(), before);
}
