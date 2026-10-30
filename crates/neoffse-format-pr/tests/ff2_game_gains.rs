//! The two FF II rules that only the game interface holds: a gain stops at
//! `cap - base` (`docs/formats/pixel-remaster.md#character-growth-in-ff-ii`,
//! "Reading a stored gain"), and no hand takes the shield placeholder
//! (`docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff2_save::{SlotData, character_field, model, standard_model, tables};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, Value};
use neoffse_format_pr::ff2::names;

const ENFORCE: CapPolicy = CapPolicy::Enforce;
const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;

/// The eight gains with the cap key of each, in the order of
/// `docs/formats/pixel-remaster.md#character-growth-in-ff-ii`.
const GAINS: [(&str, &str); 8] = [
    (names::MAX_HP_GAIN, "MAX_HP"),
    (names::MAX_MP_GAIN, "MAX_MP"),
    (names::POWER, "MAX_STRNGTH"),
    (names::VITALITY, "MAX_VITALITY"),
    (names::AGILITY, "MAX_AGILITY"),
    (names::INTELLIGENCE, "MAX_INTELIGENCE"),
    (names::SPIRIT, "MAX_SPRIT"),
    (names::MAGIC, "MAX_MAGIC"),
];

/// The base of `name` in the status row `status`, from the tables.
fn base(status: u32, name: &str) -> i64 {
    let row = tables().base_stats(status).unwrap();
    i64::from(match name {
        names::MAX_HP_GAIN => row.hp,
        names::MAX_MP_GAIN => row.mp,
        names::POWER => row.power,
        names::VITALITY => row.vitality,
        names::AGILITY => row.agility,
        names::INTELLIGENCE => row.intelligence,
        names::SPIRIT => row.spirit,
        names::MAGIC => row.magic,
        other => panic!("no gain {other}"),
    })
}

fn set(id: u32, name: &'static str, value: i64) -> Edit {
    Edit::set(character_field(id, name), Value::Int(value))
}

/// `cap - base` of a gain of character `id`, whose status row is `id`.
fn room(id: u32, name: &str, cap_key: &str) -> i64 {
    tables().bound(cap_key).unwrap() - base(id, name)
}

#[test]
fn the_gain_at_cap_minus_base_is_accepted() {
    for id in 1..=4 {
        for (name, cap_key) in GAINS {
            let room = room(id, name, cap_key);
            let mut model = standard_model();
            let change = model.apply_game(&set(id, name, room)).unwrap();
            assert_eq!(change.forward.len(), 1, "character {id} {name}");
            assert_eq!(model.get(&character_field(id, name)).unwrap(), Value::Int(room), "character {id} {name}");
        }
    }
}

#[test]
fn one_more_than_cap_minus_base_is_refused_with_that_cap() {
    for id in 1..=4 {
        for (name, cap_key) in GAINS {
            let room = room(id, name, cap_key);
            let text = SlotData::standard().render();
            let mut model = self::model(&text);
            let result = model.apply_game(&set(id, name, room + 1));
            let field = character_field(id, name);
            assert!(
                matches!(result, Err(EditError::AboveCap { field: ref named, value, cap }) if *named == field && value == room + 1 && cap == room),
                "character {id} {name}"
            );
            assert_eq!(model.document().to_json(), text, "character {id} {name}");
        }
    }
}

#[test]
fn the_rooms_are_the_ones_the_document_states() {
    // Status 1 (HP 30, MP 10, stats 10, Magic 5) and status 4 (HP 175, MP 103).
    let want_first = [9_969, 989, 89, 89, 89, 89, 89, 94];
    let want_fourth = [9_824, 896, 89, 79, 79, 83, 51, 59];
    for (index, (name, cap_key)) in GAINS.into_iter().enumerate() {
        assert_eq!(room(1, name, cap_key), want_first[index], "{name}");
        assert_eq!(room(4, name, cap_key), want_fourth[index], "{name}");
    }
}

#[test]
fn the_raw_interface_takes_a_gain_past_the_room() {
    for (name, cap_key) in GAINS {
        let room = room(1, name, cap_key);
        for value in [room + 1, i64::from(i32::MAX)] {
            let mut model = standard_model();
            assert!(model.apply(&set(1, name, value), ENFORCE).is_ok(), "{name} {value}");
        }
        let mut model = standard_model();
        assert!(model.apply(&set(1, name, 2_000_000_000), STORAGE_ONLY).is_ok(), "{name}");
        assert_eq!(model.get(&character_field(1, name)).unwrap(), Value::Int(2_000_000_000), "{name}");
    }
}

#[test]
fn the_storage_bound_answers_before_the_gain_rule() {
    // The storage bound answers first: it is the field meta, run before the rule.
    for value in [-1, i64::from(i32::MAX) + 1] {
        let result = standard_model().apply_game(&set(1, names::POWER, value));
        assert!(matches!(result, Err(EditError::OutOfRange { .. })), "{value}");
    }
}

#[test]
fn a_gain_edit_undoes_and_redoes() {
    let text = SlotData::standard().render();
    let mut model = model(&text);
    let change = model.apply_game(&set(1, names::POWER, 89)).unwrap();
    let applied = model.document().to_json();
    change.inverse.apply_with(STORAGE_ONLY, |edit, policy| model.apply(edit, policy)).unwrap();
    assert_eq!(model.document().to_json(), text);
    change.forward.apply_with(STORAGE_ONLY, |edit, policy| model.apply(edit, policy)).unwrap();
    assert_eq!(model.document().to_json(), applied);
}

#[test]
fn a_status_without_a_base_row_refuses_the_gain() {
    for status in ["11", "0", "-1"] {
        let mut data = SlotData::standard();
        data.characters[0].status_id = Some(status.to_owned());
        data.spaced = true;
        let text = data.render();
        let mut model = model(&text);
        for (name, _) in GAINS {
            let result = model.apply_game(&set(1, name, 1));
            let want: i64 = status.parse().unwrap();
            assert!(matches!(result, Err(EditError::UnknownId { id, .. }) if id == want), "{status} {name}");
            // The raw interface holds no rule, so it takes the same edit.
            assert!(model.clone().apply(&set(1, name, 1), ENFORCE).is_ok(), "{status} {name}");
        }
        assert_eq!(model.document().to_json(), text, "{status}");
        // The other characters keep their rule.
        assert!(model.apply_game(&set(2, names::POWER, 1)).is_ok());
    }
}

#[test]
fn a_character_without_a_status_key_refuses_the_gain() {
    let mut data = SlotData::standard();
    data.characters[0].omit = Some("characterStatusId");
    let mut model = model(&data.render());
    let field = character_field(1, names::POWER);
    let result = model.apply_game(&set(1, names::POWER, 1));
    assert!(matches!(result, Err(EditError::UnknownField { field: named }) if named == field));
}

#[test]
fn an_absent_character_is_a_missing_row() {
    let result = standard_model().apply_game(&set(9, names::POWER, 1));
    assert!(matches!(result, Err(EditError::MissingRow { .. })));
}

#[test]
fn the_rule_follows_the_status_not_the_id_of_the_character() {
    // Character 1 reads the status row of character 4: HP base 175.
    let mut data = SlotData::standard();
    data.characters[0].status_id = Some("4".to_owned());
    let mut model = model(&data.render());
    assert!(model.apply_game(&set(1, names::MAX_HP_GAIN, 9_824)).is_ok());
    assert!(matches!(
        model.apply_game(&set(1, names::MAX_HP_GAIN, 9_825)),
        Err(EditError::AboveCap { cap: 9_824, .. })
    ));
}

fn slot(id: u32, number: u32) -> FieldId {
    character_field(id, names::EQUIPMENT).with_id(number)
}

#[test]
fn a_shield_placeholder_is_refused_in_a_hand() {
    let mut data = SlotData::standard();
    data.spaced = true;
    let text = data.render();
    for id in 1..=4 {
        for hand in [1, 2] {
            let mut model = model(&text);
            let field = slot(id, hand);
            let result = model.apply_game(&Edit::set(field.clone(), Value::Int(208)));
            assert!(
                matches!(result, Err(EditError::UnknownId { field: ref named, id: 208 }) if *named == field),
                "character {id} hand {hand}"
            );
            assert_eq!(model.document().to_json(), text, "character {id} hand {hand}");
        }
    }
}

#[test]
fn the_other_slots_refuse_208_by_their_table_and_the_empty_hand_is_47() {
    let text = SlotData::standard().render();
    for number in 3..=5 {
        let mut model = model(&text);
        let result = model.apply_game(&Edit::set(slot(1, number), Value::Int(208)));
        assert!(matches!(result, Err(EditError::UnknownId { id: 208, .. })), "slot {number}");
        assert_eq!(model.document().to_json(), text, "slot {number}");
    }
    for hand in [1, 2] {
        assert!(standard_model().apply_game(&Edit::set(slot(3, hand), Value::Int(47))).is_ok(), "hand {hand}");
    }
}
