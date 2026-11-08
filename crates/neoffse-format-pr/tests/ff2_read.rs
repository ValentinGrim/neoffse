//! Reads of the FF II model against the synthetic slot.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::assert_document_error;
use common::character_path;
use common::ff2_save::{Character, SlotData, inventory, key_items, model, standard_model, tables};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, Value};
use neoffse_format_pr::document::{Document, DocumentError, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Model, names};

fn int(value: i64) -> Value {
    Value::Int(value)
}

fn field(id: u32, name: &'static str) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id).with_name(name)
}

/// The 32 expected values of a character, in `character_fields` order.
fn expected(character: &Character) -> Vec<Value> {
    let count_of = |content: i64| {
        inventory()
            .iter()
            .find(|(row, _)| i64::from(*row) == content)
            .map(|(_, count)| i64::from(*count))
            .unwrap()
    };
    let mut values = vec![
        Value::Text(character.name.clone()),
        int(i64::from(character.job)),
        int(character.hp),
        int(character.max_hp_gain),
        int(character.mp),
        int(character.max_mp_gain),
    ];
    values.extend(character.stats.map(int));
    values.extend(character.skills.iter().map(|(_, value)| int(*value)));
    for equip in &character.equipment {
        let content: i64 = equip.content_id.parse().unwrap();
        values.extend([int(content), int(count_of(content))]);
    }
    values
}

fn assert_characters_read(model: &mut Ff2Model, characters: &[Character]) {
    for character in characters {
        let fields = names::character_fields(character.id);
        assert_eq!(fields.len(), 32);
        for (field, want) in fields.iter().zip(expected(character)) {
            assert_eq!(model.get(field).unwrap(), want, "{field}");
        }
    }
}

#[test]
fn reads_every_declared_field_of_every_character() {
    assert_characters_read(&mut standard_model(), &SlotData::standard().characters);
}

#[test]
fn the_first_character_reads_the_values_the_generator_states() {
    let mut model = standard_model();
    let read = |model: &mut Ff2Model, name| model.get(&field(1, name)).unwrap();
    assert_eq!(read(&mut model, names::NAME), Value::Text("Ivo".to_owned()));
    assert_eq!(read(&mut model, names::JOB), int(1));
    assert_eq!((read(&mut model, names::HP), read(&mut model, names::MP)), (int(140), int(20)));
    assert_eq!(
        (read(&mut model, names::MAX_HP_GAIN), read(&mut model, names::MAX_MP_GAIN)),
        (int(120), int(14))
    );
    assert_eq!(read(&mut model, names::POWER), int(7));
    assert_eq!(read(&mut model, names::MAGIC), int(22));
    // Hand 1 and hand 2 hold the same weapon, with the inventory count.
    let hand = |slot, count: bool| {
        let id = field(1, names::EQUIPMENT).with_id(slot);
        if count { id.with_name(names::COUNT) } else { id }
    };
    assert_eq!((model.get(&hand(1, false)).unwrap(), model.get(&hand(2, false)).unwrap()), (int(60), int(60)));
    assert_eq!((model.get(&hand(1, true)).unwrap(), model.get(&hand(2, true)).unwrap()), (int(3), int(3)));
}

#[test]
fn reads_gil_counts_and_rows() {
    let mut model = standard_model();
    assert_eq!(model.get(&FieldId::new(names::GIL)).unwrap(), int(1_234_567));
    let count = |list: &'static str, key: u32| FieldId::new(list).with_id(key).with_name(names::COUNT);
    assert_eq!(model.get(&count(names::INVENTORY, 47)).unwrap(), int(10));
    assert_eq!(model.get(&count(names::INVENTORY, 60)).unwrap(), int(3));
    let key_items = key_items();
    assert_eq!(model.get(&count(names::KEY_ITEMS, key_items[0])).unwrap(), int(1));
    let rows: Vec<u32> = inventory().iter().map(|(id, _)| *id).collect();
    assert_eq!(model.rows(&FieldId::new(names::INVENTORY)).unwrap(), rows);
    assert_eq!(model.rows(&FieldId::new(names::KEY_ITEMS)).unwrap(), key_items);
    assert_eq!(model.rows(&FieldId::new(names::CHARACTER)).unwrap(), [1, 2, 3, 4]);
}

#[test]
fn lookup_is_by_id_not_by_position() {
    let mut slot = SlotData::standard();
    let all = slot.characters.clone();
    slot.characters = vec![all[2].clone(), all[0].clone(), all[3].clone(), all[1].clone()];
    let mut model = model(&slot.render());
    assert_eq!(model.rows(&FieldId::new(names::CHARACTER)).unwrap(), [3, 1, 4, 2]);
    assert_eq!(model.get(&field(1, names::NAME)).unwrap(), Value::Text("Ivo".to_owned()));
    assert_characters_read(&mut model, &all);
}

#[test]
fn a_repeated_key_reads_its_first_row() {
    let mut slot = SlotData::standard();
    let mut twin = slot.characters[1].clone();
    twin.id = 1;
    twin.hp = 5;
    slot.characters.push(twin);
    slot.characters[0].skills.push((3, 999));
    let mut model = model(&slot.render());
    assert_eq!(model.get(&field(1, names::HP)).unwrap(), int(140));
    let skill = field(1, names::SKILL).with_id(3);
    let first = SlotData::standard().characters[0].skills[3].1;
    assert_eq!(model.get(&skill).unwrap(), int(first));
}

#[test]
fn dictionary_lookup_is_by_key_not_by_position() {
    let mut slot = SlotData::standard();
    for character in &mut slot.characters {
        character.skills.reverse();
        character.equipment.reverse();
    }
    assert_characters_read(&mut model(&slot.render()), &SlotData::standard().characters);
}

#[test]
fn skill_keys_zero_to_nine_and_slots_one_to_five_read() {
    let mut model = standard_model();
    for key in 0..10 {
        assert!(model.get(&field(2, names::SKILL).with_id(key)).is_ok(), "skill {key}");
    }
    for slot in 1..=5 {
        let equipment = field(2, names::EQUIPMENT).with_id(slot);
        assert!(model.get(&equipment).is_ok(), "slot {slot}");
        assert!(model.get(&equipment.with_name(names::COUNT)).is_ok(), "slot {slot} count");
    }
}

#[test]
fn ff1_vocabulary_is_unknown_on_an_ff2_slot() {
    let mut model = standard_model();
    let level = field(1, "level");
    let exp = field(1, "exp");
    let charges = field(1, "charges").with_id(1);
    let charges_gain = field(1, "charges_max_gain").with_id(1);
    let spell = field(1, "spell").with_id(1).with_id(1);
    let accuracy = field(1, "accuracy_gain");
    let luck = field(1, "luck");
    let next = field(1, "next_level");
    for path in [level, exp, charges, charges_gain, spell, accuracy, luck, next] {
        assert!(matches!(model.get(&path), Err(EditError::UnknownField { field }) if field == path), "{path}");
        assert!(model.field_meta(&path).is_none(), "{path}");
    }
}

#[test]
fn ff1_vocabulary_cannot_be_set_on_an_ff2_slot() {
    let text = SlotData::standard().render();
    let mut model = model(&text);
    let paths = [
        field(1, "level"),
        field(1, "exp"),
        field(1, "charges").with_id(1),
        field(1, "spell").with_id(1).with_id(1),
    ];
    for path in paths {
        for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
            let result = model.apply(&Edit::set(path.clone(), int(5)), policy);
            assert!(matches!(result, Err(EditError::UnknownField { field }) if field == path), "{path}");
        }
        let result = model.apply_game(&Edit::set(path.clone(), int(5)));
        assert!(matches!(result, Err(EditError::UnknownField { field }) if field == path), "{path}");
    }
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn the_person_and_the_computed_fields_are_read_only() {
    let text = SlotData::standard().render();
    let mut model = model(&text);
    for name in [names::JOB, names::MAX_HP, names::MAX_MP] {
        let path = field(1, name);
        for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
            let result = model.apply(&Edit::set(path.clone(), int(3)), policy);
            assert!(matches!(result, Err(EditError::ReadOnly { field }) if field == path), "{path}");
        }
        let result = model.apply_game(&Edit::set(path.clone(), int(3)));
        assert!(matches!(result, Err(EditError::ReadOnly { field }) if field == path), "{path}");
    }
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn unknown_paths_are_unknown_fields() {
    let mut model = standard_model();
    let character = || FieldId::new(names::CHARACTER).with_id(1);
    let paths = [
        character().with_name("nope"),
        character().with_name(names::SKILL).with_id(10),
        character().with_name(names::SKILL),
        character().with_name(names::EQUIPMENT).with_id(0),
        character().with_name(names::EQUIPMENT).with_id(6),
        character().with_name(names::EQUIPMENT).with_id(1).with_name("nope"),
        FieldId::new(names::INVENTORY).with_id(60).with_name(names::NAME),
        FieldId::new(names::GIL).with_id(1),
        FieldId::new("nothing"),
        character(),
    ];
    for path in paths {
        assert!(matches!(model.get(&path), Err(EditError::UnknownField { field }) if field == path), "{path}");
        assert!(model.field_meta(&path).is_none(), "{path}");
    }
}

#[test]
fn absent_rows_and_keys() {
    let mut model = standard_model();
    let Err(EditError::MissingRow { row }) = model.get(&field(9, names::NAME)) else {
        panic!("no MissingRow")
    };
    assert_eq!(row, FieldId::new(names::CHARACTER).with_id(9));
    let item = FieldId::new(names::INVENTORY).with_id(999_999);
    let Err(EditError::MissingRow { row }) = model.get(&item.clone().with_name(names::COUNT)) else {
        panic!("no MissingRow")
    };
    assert_eq!(row, item);

    let mut slot = SlotData::standard();
    slot.characters[0].skills.retain(|(key, _)| *key != 3);
    slot.characters[0].equipment.retain(|equip| equip.key != 4);
    let mut model = model_of(&slot);
    for missing in [
        field(1, names::SKILL).with_id(3),
        field(1, names::EQUIPMENT).with_id(4),
        field(1, names::EQUIPMENT).with_id(4).with_name(names::COUNT),
    ] {
        assert!(
            matches!(model.get(&missing), Err(EditError::UnknownField { field }) if field == missing),
            "{missing}"
        );
    }
    assert!(model.get(&field(1, names::SKILL).with_id(4)).is_ok());
}

fn model_of(slot: &SlotData) -> Ff2Model {
    model(&slot.render())
}

#[test]
fn rows_of_another_path_are_not_a_collection() {
    let mut model = standard_model();
    let gil = FieldId::new(names::GIL);
    assert!(matches!(model.rows(&gil), Err(EditError::NotACollection { field }) if field == gil));
    let row = FieldId::new(names::CHARACTER).with_id(1);
    assert!(matches!(model.rows(&row), Err(EditError::NotACollection { .. })));
}

#[test]
fn a_document_of_another_shape_is_a_format_error() {
    let hp_path = character_path(0, &[Segment::Doc, Segment::Key("parameter"), Segment::Doc, Segment::Key("currentHP")]);
    for replacement in [Scalar::Float(140.5), Scalar::Text("x".to_owned()), Scalar::Uint(u64::MAX)] {
        let mut document = Document::parse(&SlotData::standard().render()).unwrap();
        document.set(&hp_path, &replacement).unwrap();
        let mut model = Ff2Model::new(document, tables());
        assert!(
            matches!(assert_document_error(model.get(&field(1, names::HP))), DocumentError::WrongKind { .. }),
            "{replacement:?}"
        );
    }
    let mut slot = SlotData::standard();
    slot.gil = Some("\"rich\"".to_owned());
    assert!(matches!(
        assert_document_error(model_of(&slot).get(&FieldId::new(names::GIL))),
        DocumentError::WrongKind { .. }
    ));
}
