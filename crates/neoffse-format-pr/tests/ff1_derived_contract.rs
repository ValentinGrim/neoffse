//! The computed `max_hp` and `next_level` fields as part of the model
//! contract: meta, view order, read-only refusal and the effect of an edit.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{SlotData, character_field, model, standard_model};
use common::json_diff::{diff, value};
use neoffse_core::{CapPolicy, Capability, ChangeSet, Edit, EditError, FieldId, Shape, Value};
use neoffse_data::Language;
use neoffse_format_pr::ff1::{Ff1Model, names};

const STORAGE_MAX: i64 = 2_147_483_647;
const CHARACTER_PATH: &str = "userData.ownedCharacterList.target";

fn int(model: &mut Ff1Model, id: u32, name: &'static str) -> Option<i64> {
    match model.get(&character_field(id, name)) {
        Ok(Value::Int(number)) => Some(number),
        Ok(other) => panic!("character {id}, {name}: {other:?}"),
        Err(EditError::UnknownField { .. }) => None,
        Err(error) => panic!("character {id}, {name}: {error}"),
    }
}

fn set(id: u32, name: &'static str, number: i64) -> Edit {
    Edit::set(character_field(id, name), Value::Int(number))
}

fn shown(model: &mut Ff1Model, id: u32, name: &'static str) -> Option<i64> {
    let view = model.view(Language::En).unwrap();
    let field = view.find(&character_field(id, name)).map(|(_, field)| field.value.clone());
    field.map(|value| match value {
        Value::Int(number) => number,
        other => panic!("character {id}, {name}: {other:?}"),
    })
}

fn run(model: &mut Ff1Model, set: &ChangeSet) {
    set.apply_with(CapPolicy::StorageOnly, |edit, policy| model.apply(edit, policy)).unwrap();
}

#[test]
fn meta_of_the_computed_fields() {
    let model = standard_model();
    for (name, label, cap) in [(names::MAX_HP, "Max HP", Some(999)), (names::NEXT_LEVEL, "Next level", None)] {
        let meta = model.field_meta(&character_field(1, name)).unwrap();
        assert_eq!(meta.label, label, "{name}");
        assert_eq!(meta.shape, Shape::Int { min: 0, max: STORAGE_MAX }, "{name}");
        assert_eq!(meta.cap, cap, "{name}");
        assert!(meta.read_only, "{name}");
        assert!(meta.tables.is_empty() && meta.names.is_empty(), "{name}");
    }
}

#[test]
fn the_two_fields_sit_next_to_their_sources() {
    let mut model = standard_model();
    let view = model.view(Language::En).unwrap();
    let party = view.section(Capability::Party).unwrap();
    for entity in &party.entities {
        let id = entity
            .id
            .keys()
            .iter()
            .find_map(|key| if let neoffse_core::Key::Id(id) = key { Some(*id) } else { None })
            .unwrap();
        let found: Vec<FieldId> = entity.fields.iter().map(|field| field.id.clone()).collect();
        let expected = names::party_fields(id);
        assert_eq!(found, expected, "character {id}");
        let position = |name| expected.iter().position(|field| *field == character_field(id, name)).unwrap();
        assert_eq!(position(names::NEXT_LEVEL), position(names::EXP) + 1, "character {id}");
        assert_eq!(position(names::MAX_HP), position(names::HP) + 1, "character {id}");
        assert_eq!(position(names::MAX_HP) + 1, position(names::MAX_HP_GAIN), "character {id}");
    }
    let stored = names::character_fields(1);
    let derived = names::derived_fields(1);
    assert_eq!((stored.len(), derived.len(), names::party_fields(1).len()), (41, 2, 43));
    assert!(derived.iter().all(|field| !stored.contains(field)));
    assert_eq!(derived, [character_field(1, names::MAX_HP), character_field(1, names::NEXT_LEVEL)]);
}

#[test]
fn every_view_value_equals_get_and_meta() {
    let mut model = standard_model();
    let view = model.view(Language::En).unwrap();
    for entity in &view.section(Capability::Party).unwrap().entities {
        for field in &entity.fields {
            assert_eq!(model.get(&field.id).unwrap(), field.value, "{}", field.id);
            assert_eq!(model.field_meta(&field.id), Some(field.meta.clone()), "{}", field.id);
        }
    }
}

#[test]
fn both_fields_refuse_every_set() {
    for spaced in [false, true] {
        let slot = SlotData {
            spaced,
            ..SlotData::standard()
        };
        let text = slot.render();
        let mut model = model(&text);
        for name in [names::MAX_HP, names::NEXT_LEVEL] {
            let field = character_field(1, name);
            for number in [0, 100] {
                let edit = Edit::set(field.clone(), Value::Int(number));
                for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
                    let refused = model.apply(&edit, policy);
                    assert!(matches!(refused, Err(EditError::ReadOnly { .. })), "{name}, spaced {spaced}");
                }
                assert!(matches!(model.apply_game(&edit), Err(EditError::ReadOnly { .. })), "{name}, spaced {spaced}");
            }
            assert!(model.field_locked(&field).unwrap(), "{name}");
            assert_eq!(model.document().to_json(), text, "{name}, spaced {spaced}");
        }
    }
}

#[test]
fn an_edit_moves_the_computed_values() {
    let text = SlotData::standard().render();
    let mut model = model(&text);
    let gain = model.apply_game(&set(1, names::MAX_HP_GAIN, 1000)).unwrap();
    assert_eq!(shown(&mut model, 1, names::MAX_HP), Some(999));
    assert_eq!(
        diff(&text, &model.document().to_json()),
        [value(&format!("{CHARACTER_PATH}[0].parameter.addtionalMaxHp"))]
    );
    run(&mut model, &gain.inverse);
    assert_eq!(shown(&mut model, 1, names::MAX_HP), Some(446));
    assert_eq!(model.document().to_json(), text);

    let exp = model.apply_game(&set(2, names::EXP, 100_000)).unwrap();
    assert_eq!(shown(&mut model, 2, names::NEXT_LEVEL), Some(3811));
    // The game interface also sets the level the experience reaches (31).
    assert_eq!(
        diff(&text, &model.document().to_json()),
        [
            value(&format!("{CHARACTER_PATH}[1].currentExp")),
            value(&format!("{CHARACTER_PATH}[1].parameter.addtionalLevel"))
        ]
    );
    run(&mut model, &exp.inverse);
    assert_eq!(model.document().to_json(), text);

    // A raw edit of the level links nothing: the figure follows the experience.
    for level in [40, 99] {
        let inverse = model.apply(&set(1, names::LEVEL, level), CapPolicy::Enforce).unwrap();
        assert_eq!(shown(&mut model, 1, names::NEXT_LEVEL), Some(3711), "level {level}");
        assert_eq!(
            diff(&text, &model.document().to_json()),
            [value(&format!("{CHARACTER_PATH}[0].parameter.addtionalLevel"))]
        );
        model.apply(&inverse, CapPolicy::Enforce).unwrap();
        assert_eq!(model.document().to_json(), text);
    }

    let top = model.apply_game(&set(1, names::EXP, 1_471_455)).unwrap();
    assert_eq!(shown(&mut model, 1, names::NEXT_LEVEL), None);
    // The 68 levels gained also grow the stats (`ff1_levelup.rs` holds the values).
    let changes = diff(&text, &model.document().to_json());
    assert!(changes.contains(&value(&format!("{CHARACTER_PATH}[0].currentExp"))));
    assert!(changes.contains(&value(&format!("{CHARACTER_PATH}[0].parameter.addtionalLevel"))));
    assert!(changes.len() > 2, "the growth edits are in the diff");
    run(&mut model, &top.inverse);
    assert_eq!(shown(&mut model, 1, names::NEXT_LEVEL), Some(3711));
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn the_view_changes_nothing_in_the_document() {
    let slot = SlotData {
        spaced: true,
        ..SlotData::standard()
    };
    let text = slot.render();
    let mut model = model(&text);
    model.view(Language::En).unwrap();
    assert_eq!(model.document().to_json(), text);
    assert_eq!(int(&mut model, 1, names::MAX_HP), Some(446));
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn field_locked_agrees_with_the_game_set_for_the_new_ids() {
    let mut model = standard_model();
    for field in names::derived_fields(1) {
        assert!(model.field_locked(&field).unwrap(), "{field}");
        let refused = model.apply_game(&Edit::set(field.clone(), Value::Int(1)));
        assert!(matches!(refused, Err(EditError::ReadOnly { .. })), "{field}");
    }
    let stored = character_field(1, names::MAX_HP_GAIN);
    assert!(!model.field_locked(&stored).unwrap());
}
