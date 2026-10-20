//! The raw `apply` of the FF I model for `Set`: round trip, locality and the
//! absence of game rules.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::changed_span;
use common::ff1_save::{SlotData, character_field, model, standard_model};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, Key, Value};
use neoffse_format_pr::document::{Scalar, Segment};
use neoffse_format_pr::ff1::{Ff1Model, names};

fn gil() -> FieldId {
    FieldId::new(names::GIL)
}

fn count(list: &'static str, key: u32) -> FieldId {
    FieldId::new(list).with_id(key).with_name(names::COUNT)
}

fn slot(id: u32, slot: u32) -> FieldId {
    character_field(id, names::EQUIPMENT).with_id(slot)
}

fn set(field: &FieldId, value: Value) -> Edit {
    Edit::set(field.clone(), value)
}

fn name_of(value: &Value) -> String {
    let Value::Text(text) = value else { panic!("not text") };
    text.clone()
}

/// Every declared field of the four characters, `gil` and every item count.
fn snapshot(model: &mut Ff1Model) -> Vec<(FieldId, Value)> {
    let mut fields = vec![gil()];
    for id in 1..=4 {
        fields.extend(names::character_fields(id));
    }
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        for key in model.rows(&FieldId::new(list)).unwrap() {
            fields.push(count(list, key));
        }
    }
    fields.into_iter().map(|field| (field.clone(), model.get(&field).unwrap())).collect()
}

/// A different, valid value for `field` under `Enforce`: slots take an id of
/// their own table.
fn new_value(field: &FieldId, old: &Value) -> Value {
    const SLOT_IDS: [i64; 5] = [104, 165, 160, 188, 133];
    let keys = field.keys();
    if let Value::Text(_) = old {
        return Value::Text("Renamed".to_owned());
    }
    let Value::Int(old) = old else { panic!("not an integer") };
    match keys {
        [_, _, Key::Name(names::JOB)] => Value::Int(1),
        [_, _, Key::Name(names::EQUIPMENT), Key::Id(slot)] => Value::Int(SLOT_IDS[usize::try_from(*slot).unwrap() - 1]),
        _ if *old < 5 => Value::Int(old + 3),
        _ => Value::Int(old - 3),
    }
}

#[test]
fn every_declared_field_takes_an_edit() {
    let before = snapshot(&mut standard_model());
    for field in names::character_fields(2) {
        let mut model = standard_model();
        let old = model.get(&field).unwrap();
        let value = new_value(&field, &old);
        assert_ne!(value, old, "{field}");
        model.apply(&set(&field, value.clone()), CapPolicy::Enforce).unwrap();
        assert_eq!(model.get(&field).unwrap(), value, "{field}");
        for ((other, was), (_, now)) in before.iter().zip(snapshot(&mut model)) {
            if *other != field {
                assert_eq!(*was, now, "{field} changed {other}");
            }
        }
    }
}

#[test]
fn set_then_inverse_restores_the_bytes_for_every_field() {
    let text = SlotData::standard().render();
    let mut fields: Vec<FieldId> = names::character_fields(2);
    fields.extend([gil(), count(names::INVENTORY, 2), count(names::INVENTORY, 62), count(names::KEY_ITEMS, 45)]);
    for field in fields {
        let mut model = model(&text);
        let old = model.get(&field).unwrap();
        let value = new_value(&field, &old);
        let inverse = model.apply(&set(&field, value), CapPolicy::Enforce).unwrap();
        assert_eq!(inverse, set(&field, old), "{field}");
        model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
        assert_eq!(model.document().to_json(), text, "{field}");
    }
}

#[test]
fn an_edit_changes_only_its_digits() {
    let text = SlotData::standard().render();
    let cases = [
        (character_field(1, names::LEVEL), 50),
        (character_field(1, names::EXP), 5),
        (character_field(1, names::HP), 999),
        (gil(), 7),
    ];
    for (field, value) in cases {
        let mut model = model(&text);
        model.apply(&set(&field, Value::Int(value)), CapPolicy::Enforce).unwrap();
        let after = model.document().to_json();
        let (was, now) = changed_span(&text, &after);
        for side in [was, now] {
            assert!(!side.is_empty() && side.bytes().all(|byte| byte.is_ascii_digit()), "{field}");
        }
    }
}

#[test]
fn name_keeps_utf8_quotes_and_slashes() {
    for name in ["Zoé 戦士", "say \"hi\"", "back\\slash", "<a&b>"] {
        let mut model = standard_model();
        let field = character_field(3, names::NAME);
        model.apply(&set(&field, Value::Text(name.to_owned())), CapPolicy::Enforce).unwrap();
        assert_eq!(name_of(&model.get(&field).unwrap()), name);
        let text = model.document().to_json();
        assert!(!text.contains("\\u"), "{name}");
        if name.contains('戦') || name.contains('<') {
            assert!(text.contains(name), "{name}");
        }
    }
}

#[test]
fn an_edit_keeps_play_time_and_siblings_verbatim() {
    let mut model = standard_model();
    let user = |tail: &[Segment<'static>]| {
        let mut path = vec![Segment::Key("userData"), Segment::Doc];
        path.extend_from_slice(tail);
        path
    };
    let element = |index| user(&[Segment::Key("ownedCharacterList"), Segment::Doc, Segment::Key("target"), Segment::Index(index)]);
    let paths = [
        user(&[Segment::Key("playTime")]),
        user(&[Segment::Key("normalOwnedItemList")]),
        user(&[Segment::Key("corpsList")]),
        element(0),
        element(1),
        element(3),
        vec![Segment::Key("mapData")],
    ];
    let mut before = model.document().clone();
    let raws: Vec<String> = paths.iter().map(|path| before.raw(path).unwrap().into_owned()).collect();
    assert_eq!(raws[0], "13621.814333377138");
    model.apply(&set(&gil(), Value::Int(500)), CapPolicy::Enforce).unwrap();
    let mut after = model.document().clone();
    for (path, raw) in paths.iter().zip(&raws) {
        assert_eq!(after.raw(path).unwrap(), raw.as_str());
    }
}

#[test]
fn edits_go_by_id() {
    let all = SlotData::standard().characters;
    let mut slot = SlotData::standard();
    slot.characters = vec![all[2].clone(), all[0].clone(), all[3].clone(), all[1].clone()];
    let mut model = model(&slot.render());
    let before = snapshot(&mut model);
    model
        .apply(&set(&character_field(4, names::LEVEL), Value::Int(50)), CapPolicy::Enforce)
        .unwrap();
    let level = [
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("ownedCharacterList"),
        Segment::Doc,
        Segment::Key("target"),
        Segment::Index(2),
        Segment::Doc,
        Segment::Key("parameter"),
        Segment::Doc,
        Segment::Key("addtionalLevel"),
    ];
    assert_eq!(model.read(&level).unwrap(), Scalar::Int(50));
    for ((field, was), (_, now)) in before.iter().zip(snapshot(&mut model)) {
        if *field != character_field(4, names::LEVEL) {
            assert_eq!(*was, now, "{field}");
        }
    }
}

#[test]
fn charges_and_equipment_edits_go_by_dictionary_key() {
    let mut data = SlotData::standard();
    for character in &mut data.characters {
        character.charges.reverse();
        character.equipment.reverse();
    }
    let mut model = model(&data.render());
    let character = [
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("ownedCharacterList"),
        Segment::Doc,
        Segment::Key("target"),
        Segment::Index(0),
        Segment::Doc,
    ];
    let charges = FieldId::new(names::CHARACTER).with_id(1).with_name(names::CHARGES).with_id(1);
    model.apply(&set(&charges, Value::Int(9)), CapPolicy::Enforce).unwrap();
    model.apply(&set(&slot(1, 1), Value::Int(104)), CapPolicy::Enforce).unwrap();
    model
        .apply(&set(&slot(1, 1).with_name(names::COUNT), Value::Int(6)), CapPolicy::Enforce)
        .unwrap();
    let mut path = character.to_vec();
    path.extend([
        Segment::Key("parameter"),
        Segment::Doc,
        Segment::Key("currentMpCountList"),
        Segment::Doc,
        Segment::Key("values"),
        Segment::Index(7),
    ]);
    assert_eq!(model.read(&path).unwrap(), Scalar::Int(9));
    let mut path = character.to_vec();
    path.extend([
        Segment::Key("equipmentList"),
        Segment::Doc,
        Segment::Key("values"),
        Segment::Index(4),
        Segment::Doc,
    ]);
    path.push(Segment::Key("contentId"));
    assert_eq!(model.read(&path).unwrap(), Scalar::Int(104));
    path.pop();
    path.push(Segment::Key("count"));
    assert_eq!(model.read(&path).unwrap(), Scalar::Int(6));
}

#[test]
fn a_raw_equipment_edit_derives_nothing() {
    let mut model = standard_model();
    let rows = model.rows(&FieldId::new(names::INVENTORY)).unwrap();
    model.apply(&set(&slot(1, 1), Value::Int(104)), CapPolicy::Enforce).unwrap();
    assert_eq!(model.get(&slot(1, 1).with_name(names::COUNT)).unwrap(), Value::Int(1));
    assert_eq!(model.rows(&FieldId::new(names::INVENTORY)).unwrap(), rows);
    assert_eq!(model.get(&count(names::INVENTORY, 89)).unwrap(), Value::Int(1));
    assert!(matches!(model.get(&count(names::INVENTORY, 104)), Err(EditError::MissingRow { .. })));
}

#[test]
fn raw_edits_carry_no_game_rule() {
    let text = SlotData::standard().render();
    let cases = [
        (slot(1, 1).with_name(names::COUNT), 7),
        (count(names::INVENTORY, 202), 3),
        (count(names::INVENTORY, 62), 4),
    ];
    for (field, value) in cases {
        let mut model = model(&text);
        let inverse = model.apply(&set(&field, Value::Int(value)), CapPolicy::Enforce).unwrap();
        assert_eq!(model.get(&field).unwrap(), Value::Int(value), "{field}");
        model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
        assert_eq!(model.document().to_json(), text, "{field}");
    }
}

#[test]
fn a_bare_slot_holding_zero_can_be_set_and_undone() {
    let mut data = SlotData::standard();
    data.characters[2].equipment[2].content_id = "0".to_owned();
    data.characters[2].equipment[2].count = Some("0".to_owned());
    let text = data.render();
    let mut model = model(&text);
    let inverse = model.apply(&set(&slot(3, 3), Value::Int(154)), CapPolicy::Enforce).unwrap();
    assert_eq!(inverse, set(&slot(3, 3), Value::Int(0)));
    assert_eq!(model.get(&slot(3, 3)).unwrap(), Value::Int(154));
    model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn two_edits_in_different_branches_both_hold() {
    let mut model = standard_model();
    model.apply(&set(&gil(), Value::Int(42)), CapPolicy::Enforce).unwrap();
    model
        .apply(&set(&character_field(4, names::NAME), Value::Text("Dora".to_owned())), CapPolicy::Enforce)
        .unwrap();
    model.apply(&set(&count(names::KEY_ITEMS, 46), Value::Int(3)), CapPolicy::Enforce).unwrap();
    assert_eq!(model.get(&gil()).unwrap(), Value::Int(42));
    assert_eq!(name_of(&model.get(&character_field(4, names::NAME)).unwrap()), "Dora");
    assert_eq!(model.get(&count(names::KEY_ITEMS, 46)).unwrap(), Value::Int(3));
    assert_eq!(model.get(&character_field(1, names::NAME)).unwrap(), Value::Text("Aria".to_owned()));
}
