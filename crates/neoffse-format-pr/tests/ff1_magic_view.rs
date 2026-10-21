//! The Magic section, the meta and the read-only rule of the FF I spell
//! slots (`docs/formats/pixel-remaster.md#abilities`, "FF I spell slots").
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::embed;
use common::ff1_format::{open, standard_files};
use common::ff1_magic::{ability, level_element, model_with_slots, spell};
use common::ff1_save::{SlotData, model, standard_model};
use neoffse_core::{CapPolicy, Capability, Edit, EditError, FieldId, FormatError, NamedValue, SaveDocument, Shape, TableKind, Value};
use neoffse_data::Language;
use neoffse_format_pr::container::Container;
use neoffse_format_pr::ff1::{CAPABILITIES, names};

const STORAGE_MAX: i64 = 2_147_483_647;

fn set(field: &FieldId, value: i64) -> Edit {
    Edit::set(field.clone(), Value::Int(value))
}

#[test]
fn the_view_has_party_inventory_then_magic() {
    let view = standard_model().view(Language::En).unwrap();
    let found: Vec<Capability> = view.sections.iter().map(|section| section.capability).collect();
    assert_eq!(found, [Capability::Party, Capability::Inventory, Capability::Magic]);
    assert_eq!(found, CAPABILITIES);
}

#[test]
fn the_document_reports_the_three_capabilities() {
    let document = open(&standard_files(&Container::game()));
    assert_eq!(document.capabilities(), CAPABILITIES);
}

#[test]
fn magic_lists_one_entity_per_character_with_the_party_label() {
    let mut model = standard_model();
    let view = model.view(Language::En).unwrap();
    let party = view.section(Capability::Party).unwrap();
    let magic = view.section(Capability::Magic).unwrap();
    assert_eq!(magic.collections.len(), 0);
    assert_eq!(magic.entities.len(), party.entities.len());
    for (entity, member) in magic.entities.iter().zip(&party.entities) {
        assert_eq!(entity.id, member.id);
        assert_eq!(entity.label, member.label);
        assert_eq!(entity.fields.len(), 24, "{}", entity.id);
        for field in &entity.fields {
            assert_eq!(Some(field.meta.clone()), model.field_meta(&field.id), "{}", field.id);
            assert_eq!(model.get(&field.id).unwrap(), field.value, "{}", field.id);
        }
    }
}

#[test]
fn magic_fields_follow_spell_fields_order() {
    let view = standard_model().view(Language::En).unwrap();
    let magic = view.section(Capability::Magic).unwrap();
    for (index, entity) in magic.entities.iter().enumerate() {
        let id = u32::try_from(index).unwrap() + 1;
        let found: Vec<FieldId> = entity.fields.iter().map(|field| field.id.clone()).collect();
        assert_eq!(found, names::spell_fields(id));
    }
}

#[test]
fn an_empty_name_gets_the_generic_label_in_magic_too() {
    let mut slot = SlotData::standard();
    slot.characters[1].name = String::new();
    let view = model(&slot.render()).view(Language::En).unwrap();
    let magic = view.section(Capability::Magic).unwrap();
    assert_eq!(magic.entities[1].label, "Character 2");
    assert_eq!(magic.entities[0].label, "Aria");
}

#[test]
fn an_absent_slot_is_left_out_of_the_view() {
    let elements = [
        level_element(1, &[1, 2], &[&ability(5), "\"\""]),
        level_element(2, &[1, 2, 3], &["\"\"", "\"\"", "\"\""]),
    ];
    let (mut model, _) = model_with_slots(3, &elements);
    let view = model.view(Language::En).unwrap();
    let entity = &view.section(Capability::Magic).unwrap().entities[2];
    assert_eq!(entity.fields.len(), 2 + 3);
    assert!(entity.fields.iter().all(|field| field.id != spell(3, 1, 3)));
    let other = &view.section(Capability::Magic).unwrap().entities[0];
    assert_eq!(other.fields.len(), 24);
}

#[test]
fn a_slot_of_another_shape_fails_the_view() {
    for value in ["1".to_owned(), embed("{\"contentId\":213}")] {
        let elements = [level_element(1, &[1, 2, 3], &[&value, "\"\"", "\"\""])];
        let (mut model, _) = model_with_slots(3, &elements);
        assert!(matches!(model.view(Language::En), Err(FormatError::Decode { .. })), "{value}");
    }
}

#[test]
fn a_spell_meta_is_read_only_with_the_ability_table() {
    let model = standard_model();
    let mut labels = std::collections::HashSet::new();
    for field in names::spell_fields(2) {
        let meta = model.field_meta(&field).unwrap();
        assert!(meta.read_only, "{field}");
        assert_eq!(meta.shape, Shape::Int { min: 0, max: STORAGE_MAX }, "{field}");
        assert_eq!(meta.cap, None, "{field}");
        assert_eq!(meta.tables, [TableKind::Ability], "{field}");
        assert_eq!(
            meta.names,
            [NamedValue {
                value: 0,
                name: "Empty".to_owned()
            }],
            "{field}"
        );
        assert!(labels.insert(meta.label.clone()), "{field}: label repeats");
    }
    assert_eq!(model.field_meta(&spell(2, 1, 1)).unwrap().label, "Level 1, slot 1");
    assert_eq!(model.field_meta(&spell(2, 8, 3)).unwrap().label, "Level 8, slot 3");
}

#[test]
fn a_raw_set_is_read_only_under_both_policies_and_changes_nothing() {
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        let mut model = standard_model();
        let before = model.document().to_json();
        for field in [spell(3, 1, 1), spell(3, 1, 3), spell(1, 8, 3)] {
            for value in [0, 5, 6, -1, i64::MAX] {
                let result = model.apply(&set(&field, value), policy);
                assert!(matches!(&result, Err(EditError::ReadOnly { field: named }) if *named == field), "{field}");
            }
            let text = model.apply(&Edit::set(field.clone(), Value::Text("x".to_owned())), policy);
            assert!(matches!(text, Err(EditError::ReadOnly { .. })), "{field}");
        }
        assert_eq!(model.document().to_json(), before);
    }
}

#[test]
fn a_raw_set_stays_read_only_on_a_slot_of_another_shape() {
    let elements = [level_element(1, &[1, 2, 3], &["1", "\"\"", "\"\""])];
    let (mut model, text) = model_with_slots(3, &elements);
    for field in [spell(3, 1, 1), spell(3, 5, 1)] {
        let result = model.apply(&set(&field, 5), CapPolicy::StorageOnly);
        assert!(matches!(result, Err(EditError::ReadOnly { .. })), "{field}");
    }
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn the_game_interface_refuses_a_spell_slot_and_changes_nothing() {
    let mut model = standard_model();
    let before = model.document().to_json();
    for field in names::spell_fields(3) {
        assert!(model.field_locked(&field).unwrap(), "{field}");
        let result = model.apply_game(&set(&field, 5));
        assert!(matches!(&result, Err(EditError::ReadOnly { field: named }) if *named == field), "{field}");
    }
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn only_declared_spell_fields_resolve_for_field_locked() {
    let model = standard_model();
    let field = FieldId::new(names::CHARACTER).with_id(1).with_name(names::SPELL).with_id(9).with_id(1);
    assert!(matches!(model.field_locked(&field), Err(EditError::UnknownField { .. })));
    assert!(!model.field_locked(&FieldId::new(names::CHARACTER).with_id(1).with_name(names::LEVEL)).unwrap());
}
