//! Reads of the FF I model against the synthetic slot.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{SlotData, model, standard_model};
use neoffse_core::{EditError, FieldId, Value};
use neoffse_format_pr::document::{DocumentError, Scalar, Segment};
use neoffse_format_pr::ff1::{Ff1Model, names};

const JOBS: [i64; 4] = [7, 8, 11, 12];
const LEVELS: [i64; 4] = [31, 32, 33, 34];
const EXPERIENCE: [i64; 4] = [100_100, 200_200, 300_300, 400_400];
const HP: [i64; 4] = [311, 322, 333, 344];
const GAINS: [i64; 4] = [411, 422, 433, 444];
const STATS: [[i64; 7]; 4] = [
    [11, 21, 31, 41, 51, 61, 71],
    [12, 22, 32, 42, 52, 62, 72],
    [13, 23, 33, 43, 53, 63, 73],
    [14, 24, 34, 44, 54, 64, 74],
];
const NAMES: [&str; 4] = ["Aria", "Bram", "Cleo", "Zoé"];
const EQUIPMENT: [[i64; 5]; 4] = [
    [89, 130, 154, 183, 202],
    [92, 170, 158, 187, 202],
    [116, 173, 160, 188, 202],
    [67, 166, 151, 183, 202],
];
/// The inventory count of each worn id, which each slot `count` copies.
const WORN_COUNTS: [(i64, i64); 16] = [
    (89, 1),
    (92, 1),
    (116, 1),
    (67, 1),
    (130, 9),
    (170, 1),
    (173, 1),
    (166, 1),
    (154, 1),
    (158, 1),
    (160, 1),
    (151, 1),
    (183, 2),
    (187, 1),
    (188, 1),
    (202, 4),
];
const INVENTORY_ROWS: [u32; 29] = [
    62, 130, 131, 132, 133, 2, 3, 5, 10, 17, 18, 89, 92, 116, 67, 170, 173, 166, 154, 158, 160, 151, 183, 187, 188, 202, 100, 103, 165,
];

fn int(value: i64) -> Value {
    Value::Int(value)
}

/// The 39 expected values of character `id` (1 to 4), in `character_fields` order.
fn expected(id: u32) -> Vec<Value> {
    let index = usize::try_from(id).unwrap() - 1;
    let id = i64::from(id);
    let mut values = vec![
        Value::Text(NAMES[index].to_owned()),
        int(JOBS[index]),
        int(LEVELS[index]),
        int(EXPERIENCE[index]),
        int(HP[index]),
        int(GAINS[index]),
    ];
    values.extend(STATS[index].map(int));
    values.extend((1..=8).map(|level| int((level + id) % 10)));
    values.extend((1..=8).map(|level| int((2 * level + id) % 8)));
    for slot in EQUIPMENT[index] {
        let count = WORN_COUNTS.iter().find(|(content, _)| *content == slot).unwrap().1;
        values.extend([int(slot), int(count)]);
    }
    values
}

fn assert_characters_read(model: &mut Ff1Model, ids: &[u32]) {
    for &id in ids {
        let fields = names::character_fields(id);
        assert_eq!(fields.len(), 39);
        for (field, want) in fields.iter().zip(expected(id)) {
            assert_eq!(model.get(field).unwrap(), want, "{field}");
        }
    }
}

fn field(id: u32, name: &'static str) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id).with_name(name)
}

#[test]
fn reads_every_declared_field_of_every_character() {
    assert_characters_read(&mut standard_model(), &[1, 2, 3, 4]);
}

#[test]
fn reads_gil_counts_and_rows() {
    let mut model = standard_model();
    assert_eq!(model.get(&FieldId::new(names::GIL)).unwrap(), int(123_456));
    let count = |list: &'static str, key: u32| FieldId::new(list).with_id(key).with_name(names::COUNT);
    assert_eq!(model.get(&count(names::INVENTORY, 89)).unwrap(), int(1));
    assert_eq!(model.get(&count(names::INVENTORY, 202)).unwrap(), int(4));
    assert_eq!(model.get(&count(names::KEY_ITEMS, 45)).unwrap(), int(1));
    assert_eq!(model.rows(&FieldId::new(names::INVENTORY)).unwrap(), INVENTORY_ROWS);
    assert_eq!(model.rows(&FieldId::new(names::KEY_ITEMS)).unwrap(), [45, 46, 48, 49, 50, 51, 52, 53, 61]);
    assert_eq!(model.rows(&FieldId::new(names::CHARACTER)).unwrap(), [1, 2, 3, 4]);
}

#[test]
fn lookup_is_by_id_not_by_position() {
    let mut slot = SlotData::standard();
    let all = slot.characters.clone();
    slot.characters = vec![all[2].clone(), all[0].clone(), all[3].clone(), all[1].clone()];
    let mut model = model(&slot.render());
    assert_eq!(model.rows(&FieldId::new(names::CHARACTER)).unwrap(), [3, 1, 4, 2]);
    assert_eq!(model.get(&field(1, names::NAME)).unwrap(), Value::Text("Aria".to_owned()));
    assert_characters_read(&mut model, &[1, 2, 3, 4]);

    let mut slot = SlotData::standard();
    slot.characters[0].name = "Twin".to_owned();
    slot.characters[1].name = "Twin".to_owned();
    let mut model = model_of(&slot);
    assert_eq!(model.get(&field(1, names::NAME)).unwrap(), model.get(&field(2, names::NAME)).unwrap());
    assert_eq!(model.get(&field(1, names::LEVEL)).unwrap(), int(31));
    assert_eq!(model.get(&field(2, names::LEVEL)).unwrap(), int(32));
}

fn model_of(slot: &SlotData) -> Ff1Model {
    model(&slot.render())
}

#[test]
fn dictionary_lookup_is_by_key_not_by_position() {
    let mut slot = SlotData::standard();
    for character in &mut slot.characters {
        character.charges.reverse();
        character.charges_gain.reverse();
        character.equipment.reverse();
    }
    assert_characters_read(&mut model_of(&slot), &[1, 2, 3, 4]);
}

#[test]
fn unknown_paths_are_unknown_fields() {
    let mut model = standard_model();
    let character = || FieldId::new(names::CHARACTER).with_id(1);
    let paths = [
        character().with_name("nope"),
        character().with_name(names::CHARGES).with_id(0),
        character().with_name(names::CHARGES).with_id(9),
        character().with_name(names::EQUIPMENT).with_id(0),
        character().with_name(names::EQUIPMENT).with_id(6),
        character().with_name(names::EQUIPMENT).with_id(1).with_name("nope"),
        FieldId::new(names::INVENTORY).with_id(89).with_name(names::NAME),
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
    let item = FieldId::new(names::INVENTORY).with_id(999);
    let Err(EditError::MissingRow { row }) = model.get(&item.clone().with_name(names::COUNT)) else {
        panic!("no MissingRow")
    };
    assert_eq!(row, item);

    let mut slot = SlotData::standard();
    slot.characters[0].charges.retain(|(level, _)| *level != 3);
    let mut model = model_of(&slot);
    let missing = field(1, names::CHARGES).with_id(3);
    assert!(matches!(model.get(&missing), Err(EditError::UnknownField { field }) if field == missing));
    assert_eq!(model.get(&field(1, names::CHARGES).with_id(4)).unwrap(), int(5));
}

#[test]
fn rows_of_another_path_are_not_a_collection() {
    let mut model = standard_model();
    let gil = FieldId::new(names::GIL);
    assert!(matches!(model.rows(&gil), Err(EditError::NotACollection { field }) if field == gil));
    let row = FieldId::new(names::CHARACTER).with_id(1);
    assert!(matches!(model.rows(&row), Err(EditError::NotACollection { .. })));
}

fn assert_document_error(result: Result<Value, EditError>) -> DocumentError {
    let Err(EditError::Format { source, .. }) = result else {
        panic!("no Format error")
    };
    *source.downcast::<DocumentError>().expect("the source is a DocumentError")
}

#[test]
fn a_document_of_another_shape_is_a_format_error() {
    let mut slot = SlotData::standard();
    slot.with_user_data = false;
    assert_document_error(model_of(&slot).get(&FieldId::new(names::GIL)));
    assert_document_error(model_of(&slot).get(&field(1, names::NAME)));

    let mut slot = SlotData::standard();
    slot.gil = None;
    let error = assert_document_error(model_of(&slot).get(&FieldId::new(names::GIL)));
    assert!(matches!(error, DocumentError::MissingKey { .. }));

    let mut slot = SlotData::standard();
    slot.character_list = Some(r#"{"target":5}"#.to_owned());
    let mut model = model_of(&slot);
    assert_document_error(model.get(&field(1, names::NAME)));
    let Err(EditError::Format { source, .. }) = model.rows(&FieldId::new(names::CHARACTER)) else {
        panic!("no Format error")
    };
    assert!(source.downcast_ref::<DocumentError>().is_some());
}

#[test]
fn a_float_or_an_oversized_integer_is_a_format_error() {
    for text in ["31.0", "9223372036854775808", "18446744073709551616", "\"31\""] {
        let mut slot = SlotData::standard();
        slot.characters[0].level = text.to_owned();
        let error = assert_document_error(model_of(&slot).get(&field(1, names::LEVEL)));
        assert!(
            matches!(error, DocumentError::WrongKind { .. } | DocumentError::NumberOutOfRange { .. }),
            "{text}"
        );
    }
}

#[test]
fn reads_leave_the_text_byte_for_byte() {
    let mut slot = SlotData::standard();
    slot.spaced = true;
    let text = slot.render();
    let mut model = model(&text);
    assert_characters_read(&mut model, &[1, 2, 3, 4]);
    model.get(&FieldId::new(names::GIL)).unwrap();
    for collection in [names::CHARACTER, names::INVENTORY, names::KEY_ITEMS] {
        model.rows(&FieldId::new(collection)).unwrap();
    }
    model.read(&[Segment::Key("id")]).unwrap();
    model.read(&[Segment::Key("userData"), Segment::Doc, Segment::Key("playTime")]).unwrap();
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn read_passes_unmodeled_paths_through() {
    let mut model = standard_model();
    assert_eq!(model.read(&[Segment::Key("id")]).unwrap(), Scalar::Int(1));
    let play_time = [Segment::Key("userData"), Segment::Doc, Segment::Key("playTime")];
    assert!(matches!(model.read(&play_time).unwrap(), Scalar::Float(_)));
    assert!(matches!(model.read(&[Segment::Key("nothing")]), Err(DocumentError::MissingKey { .. })));
}

#[test]
fn the_model_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<Ff1Model>();
}
