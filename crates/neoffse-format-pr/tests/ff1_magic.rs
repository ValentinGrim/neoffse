//! Reading the FF I spell slots: `character.<id>.spell.<level>.<slot>`
//! (`docs/formats/pixel-remaster.md#abilities`, "FF I spell slots").
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::embed;
use common::ff1_magic::{ability, level_element, model_with_slots, slot_mismatches, slotted, spell};
use common::ff1_save::{SlotData, standard_model};
use neoffse_core::{EditError, FieldId, Value};
use neoffse_format_pr::document::DocumentError;
use neoffse_format_pr::ff1::names;

const EMPTY: &str = "\"\"";

fn is_format(error: &EditError, field: &FieldId) -> bool {
    matches!(error, EditError::Format { field: named, source }
        if named == field && source.downcast_ref::<DocumentError>().is_some())
}

#[test]
fn spell_fields_lists_24_ids_by_level_then_slot() {
    let fields = names::spell_fields(1);
    assert_eq!(fields.len(), 24);
    assert_eq!(fields[0].to_string(), "character.1.spell.1.1");
    assert_eq!(fields[1].to_string(), "character.1.spell.1.2");
    assert_eq!(fields[3].to_string(), "character.1.spell.2.1");
    assert_eq!(fields[23].to_string(), "character.1.spell.8.3");
    assert_eq!(fields[0], spell(1, 1, 1));
    assert_eq!(names::SPELL, "spell");
    assert_eq!(names::SPELL_SLOTS, 3);
}

#[test]
fn the_fixture_slots_hold_the_character_spells() {
    let mut model = standard_model();
    for level in 1..=8 {
        for slot in 1..=3 {
            let want = match (level, slot) {
                (1, 1) => 5,
                (1, 2) => 6,
                _ => 0,
            };
            let field = spell(3, level, slot);
            assert_eq!(model.get(&field).unwrap(), Value::Int(want), "{field}");
        }
    }
    for id in [1, 2, 4] {
        for field in names::spell_fields(id) {
            assert_eq!(model.get(&field).unwrap(), Value::Int(0), "{field}");
        }
    }
}

#[test]
fn the_slots_equal_the_ability_list_on_the_fixture() {
    let text = SlotData::standard().render();
    let mut model = standard_model();
    assert_eq!(slot_mismatches(&mut model, &text), Vec::<String>::new());
    assert_eq!(slotted(&mut model, 3).unwrap().into_iter().collect::<Vec<_>>(), [5, 6]);
}

#[test]
fn a_slot_that_the_ability_list_lacks_is_a_mismatch() {
    let elements = [level_element(1, &[1, 2, 3], &[&ability(5), &ability(6), &ability(7)])];
    let (mut model, text) = model_with_slots(3, &elements);
    assert_eq!(slot_mismatches(&mut model, &text), ["character.3: spell slots differ from abilityList"]);
}

#[test]
fn a_level_is_found_by_value_not_by_index() {
    let mut elements: Vec<String> = (1..=8)
        .map(|level| match level {
            1 => level_element(1, &[1, 2, 3], &[&ability(5), EMPTY, &ability(6)]),
            _ => level_element(level, &[1, 2, 3], &[EMPTY, EMPTY, EMPTY]),
        })
        .collect();
    elements.reverse();
    let (mut model, _) = model_with_slots(3, &elements);
    assert_eq!(model.get(&spell(3, 1, 1)).unwrap(), Value::Int(5));
    assert_eq!(model.get(&spell(3, 1, 2)).unwrap(), Value::Int(0));
    assert_eq!(model.get(&spell(3, 1, 3)).unwrap(), Value::Int(6));
    assert_eq!(model.get(&spell(3, 8, 1)).unwrap(), Value::Int(0));
}

#[test]
fn a_slot_is_found_by_its_key_not_by_position() {
    let elements = [level_element(1, &[3, 1, 2], &[&ability(5), EMPTY, &ability(6)])];
    let (mut model, _) = model_with_slots(3, &elements);
    assert_eq!(model.get(&spell(3, 1, 3)).unwrap(), Value::Int(5));
    assert_eq!(model.get(&spell(3, 1, 1)).unwrap(), Value::Int(0));
    assert_eq!(model.get(&spell(3, 1, 2)).unwrap(), Value::Int(6));
}

#[test]
fn undeclared_spell_ids_are_unknown_fields() {
    let mut model = standard_model();
    let character = || FieldId::new(names::CHARACTER).with_id(1).with_name(names::SPELL);
    let unknown = [
        character().with_id(0).with_id(1),
        character().with_id(9).with_id(1),
        character().with_id(1).with_id(0),
        character().with_id(1).with_id(4),
        character().with_id(1),
        character().with_id(1).with_id(1).with_name("nope"),
    ];
    for field in unknown {
        let result = model.get(&field);
        assert!(matches!(&result, Err(EditError::UnknownField { field: named }) if *named == field), "{field}");
        assert!(model.field_meta(&field).is_none(), "{field}");
    }
}

#[test]
fn a_missing_level_or_slot_key_is_an_absent_field() {
    let elements = [
        level_element(1, &[1, 2], &[&ability(5), EMPTY]),
        level_element(3, &[1, 2, 3], &[EMPTY, EMPTY, EMPTY]),
    ];
    let (mut model, _) = model_with_slots(3, &elements);
    assert_eq!(model.get(&spell(3, 1, 1)).unwrap(), Value::Int(5));
    for field in [spell(3, 1, 3), spell(3, 2, 1), spell(3, 8, 3)] {
        let result = model.get(&field);
        assert!(matches!(&result, Err(EditError::UnknownField { field: named }) if *named == field), "{field}");
    }
    assert_eq!(model.get(&spell(3, 3, 2)).unwrap(), Value::Int(0));
}

#[test]
fn a_missing_character_is_a_missing_row() {
    let result = standard_model().get(&spell(9, 1, 1));
    assert!(matches!(result, Err(EditError::MissingRow { .. })));
}

#[test]
fn a_slot_of_another_shape_is_a_format_error() {
    let bad = [
        "1".to_owned(),
        embed("{\"contentId\":213}"),
        embed("{\"abilityId\":5.5}"),
        embed("{\"abilityId\":\"5\"}"),
        embed("{\"abilityId\":4294967296}"),
        embed("{\"abilityId\":-1}"),
        embed("not a document"),
        "true".to_owned(),
    ];
    for value in bad {
        let elements = [level_element(1, &[1, 2, 3], &[&value, EMPTY, EMPTY])];
        let (mut model, _) = model_with_slots(3, &elements);
        let field = spell(3, 1, 1);
        let error = model.get(&field).unwrap_err();
        assert!(is_format(&error, &field), "{field}: {error:?}");
    }
}

#[test]
fn a_bad_level_element_is_a_format_error() {
    let element = embed("{\"level\":\"one\",\"slotInfo\":\"\"}");
    let (mut model, _) = model_with_slots(3, &[element]);
    let field = spell(3, 1, 1);
    let error = model.get(&field).unwrap_err();
    assert!(is_format(&error, &field), "{field}: {error:?}");
}

#[test]
fn get_changes_no_byte() {
    let mut model = standard_model();
    let before = model.document().to_json();
    for field in names::spell_fields(3) {
        model.get(&field).unwrap();
    }
    assert_eq!(model.document().to_json(), before);
}
