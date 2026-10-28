//! The two level-up gain fields, `accuracy_gain` and `magic_evasion_gain`,
//! against the synthetic slot (`docs/formats/pixel-remaster.md#characters`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{SlotData, character_field, model, standard_model};
use common::{changed_span, character_path};
use neoffse_core::{CapPolicy, Capability, Edit, EditError, FieldId, Shape, Value};
use neoffse_data::Language;
use neoffse_format_pr::document::Segment;
use neoffse_format_pr::ff1::{Ff1Model, names};

const STORAGE_MAX: i64 = 2_147_483_647;
const ACCURACY: &str = "addtionalAccuracyRate";
const EVASION: &str = "addtionalAbilityEvasionRate";

/// `(field name, document key, stored value of character 1 to 4)`.
fn fields() -> [(&'static str, &'static str, [i64; 4]); 2] {
    [
        (names::ACCURACY_GAIN, ACCURACY, [143, 146, 149, 152]),
        (names::MAGIC_EVASION_GAIN, EVASION, [133, 136, 139, 142]),
    ]
}

/// Every `parameter` key of the synthetic character that this file reads or
/// could confuse with a gain field.
const PARAMETER_KEYS: [&str; 10] = [
    "addtionalMaxHp",
    "addtionalMagic",
    "addtionalLuck",
    ACCURACY,
    "addtionalEvasionRate",
    "addtionalAbilityDefenseRate",
    EVASION,
    "addtionalAbilityDisturbedRate",
    "addtionalCriticalRate",
    "addtionalAttack",
];

/// The stored tokens of `PARAMETER_KEYS` for the first character.
fn tokens(model: &Ff1Model) -> Vec<String> {
    let mut document = model.document().clone();
    PARAMETER_KEYS
        .iter()
        .map(|key| {
            let path = character_path(0, &[Segment::Doc, Segment::Key("parameter"), Segment::Doc, Segment::Key(key)]);
            document.raw(&path).unwrap().into_owned()
        })
        .collect()
}

/// Asserts that only `key` changed between `before` and `after`, to `new`.
fn only_changed(before: &[String], after: &[String], key: &str, new: &str) {
    for ((name, was), now) in PARAMETER_KEYS.iter().zip(before).zip(after) {
        if *name == key {
            assert_eq!(now, new, "{name}");
        } else {
            assert_eq!(was, now, "{key} changed {name}");
        }
    }
}

fn set(field: &FieldId, value: i64) -> Edit {
    Edit::set(field.clone(), Value::Int(value))
}

#[test]
fn both_fields_read_the_stored_values() {
    let mut model = standard_model();
    let view = model.view(Language::En).unwrap();
    let party = view.section(Capability::Party).unwrap();
    for (name, _, expected) in fields() {
        for (index, id) in (1..=4_u32).enumerate() {
            let field = character_field(id, name);
            assert_eq!(model.get(&field).unwrap(), Value::Int(expected[index]), "{field}");
            let shown = party.entities[index].fields.iter().find(|view| view.id == field);
            assert_eq!(shown.map(|view| view.value.clone()), Some(Value::Int(expected[index])), "{field}");
        }
    }
}

#[test]
fn meta_of_the_two_fields() {
    let model = standard_model();
    for (name, label) in [(names::ACCURACY_GAIN, "Accuracy gain"), (names::MAGIC_EVASION_GAIN, "Magic evasion gain")] {
        let meta = model.field_meta(&character_field(1, name)).unwrap();
        assert_eq!(meta.label, label);
        assert_eq!(meta.shape, Shape::Int { min: 0, max: STORAGE_MAX });
        assert_eq!(meta.cap, None, "{name}");
        assert!(!meta.read_only, "{name}");
        assert!(meta.tables.is_empty() && meta.names.is_empty(), "{name}");
    }
}

#[test]
fn the_fields_sit_after_magic() {
    for fields in [names::character_fields(1), names::party_fields(1)] {
        let at = |name| fields.iter().position(|field| *field == character_field(1, name)).unwrap();
        assert_eq!(at(names::ACCURACY_GAIN), at(names::MAGIC) + 1);
        assert_eq!(at(names::MAGIC_EVASION_GAIN), at(names::MAGIC) + 2);
    }
    assert_eq!((names::character_fields(1).len(), names::party_fields(1).len()), (41, 43));
}

#[test]
fn a_set_changes_one_token() {
    for (name, key, stored) in fields() {
        let field = character_field(1, name);
        for how in 0..3 {
            let mut model = standard_model();
            let before = model.document().to_json();
            let tokens_before = tokens(&model);
            let edit = set(&field, 7777);
            match how {
                0 => drop(model.apply(&edit, CapPolicy::Enforce).unwrap()),
                1 => drop(model.apply(&edit, CapPolicy::StorageOnly).unwrap()),
                _ => drop(model.apply_game(&edit).unwrap()),
            }
            let after = model.document().to_json();
            only_changed(&tokens_before, &tokens(&model), key, "7777");
            assert_eq!(changed_span(&before, &after), (stored[0].to_string().as_str(), "7777"), "{name} {how}");
        }
    }
}

#[test]
fn an_inverse_restores_the_text() {
    for (name, _, _) in fields() {
        let field = character_field(2, name);
        let mut model = standard_model();
        let before = model.document().to_json();
        let inverse = model.apply(&set(&field, 5), CapPolicy::Enforce).unwrap();
        model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
        assert_eq!(model.document().to_json(), before, "{name}");
        let change = model.apply_game(&set(&field, 5)).unwrap();
        change
            .inverse
            .apply_with(CapPolicy::StorageOnly, |edit, policy| model.apply(edit, policy))
            .unwrap();
        assert_eq!(model.document().to_json(), before, "{name}");
    }
}

#[test]
fn a_neighbouring_key_is_not_touched() {
    // Rendered values of character 1: `id * 3` plus 150 and 190.
    for (name, key, _) in fields() {
        let mut model = standard_model();
        let before = tokens(&model);
        model.apply(&set(&character_field(1, name), 9), CapPolicy::Enforce).unwrap();
        let after = tokens(&model);
        only_changed(&before, &after, key, "9");
        for (neighbour, value) in [("addtionalEvasionRate", "153"), ("addtionalAbilityDefenseRate", "193")] {
            let at = PARAMETER_KEYS.iter().position(|key| *key == neighbour).unwrap();
            assert_eq!(after[at], value, "{name} changed {neighbour}");
        }
    }
}

#[test]
fn storage_bounds_hold() {
    for (name, _, _) in fields() {
        let field = character_field(1, name);
        for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
            for value in [-1, STORAGE_MAX + 1] {
                let mut model = standard_model();
                let error = model.apply(&set(&field, value), policy).unwrap_err();
                assert!(
                    matches!(error, EditError::OutOfRange { min: 0, max: STORAGE_MAX, .. }),
                    "{name} {value}: {error}"
                );
            }
            let mut model = standard_model();
            model.apply(&set(&field, STORAGE_MAX), policy).unwrap();
            assert_eq!(model.get(&field).unwrap(), Value::Int(STORAGE_MAX));
        }
    }
}

#[test]
fn a_missing_key_leaves_the_field_out() {
    let mut data = SlotData::standard();
    data.characters[0].omit = Some(ACCURACY);
    let mut model = model(&data.render());
    let missing = character_field(1, names::ACCURACY_GAIN);
    let error = model.get(&missing).unwrap_err();
    assert!(matches!(error, EditError::UnknownField { .. } | EditError::Format { .. }), "{error}");
    let view = model.view(Language::En).unwrap();
    let party = view.section(Capability::Party).unwrap();
    assert!(party.entities[0].fields.iter().all(|field| field.id != missing));
    let evasion = character_field(1, names::MAGIC_EVASION_GAIN);
    assert!(party.entities[0].fields.iter().any(|field| field.id == evasion));
    assert_eq!(model.get(&evasion).unwrap(), Value::Int(133));
    assert_eq!(party.entities[0].fields.len(), 42);
}

#[test]
fn the_fields_are_not_locked() {
    for spaced in [false, true] {
        let mut data = SlotData::standard();
        data.spaced = spaced;
        let model = model(&data.render());
        for (name, _, _) in fields() {
            assert!(!model.field_locked(&character_field(1, name)).unwrap(), "{name} spaced {spaced}");
        }
    }
}
