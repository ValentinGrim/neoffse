//! The computed `max_hp` and `next_level` fields against the synthetic slot.
//! The expected numbers are hand-computed from the base HP and experience
//! tables of `docs/formats/pixel-remaster.md#maximum-hp-and-experience-tables`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_path;
use common::ff1_save::{SlotData, character_field, model, standard_model};
use neoffse_core::{Capability, EditError, FormatError, SaveView, Value};
use neoffse_data::Language;
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff1::{Ff1Model, GAME, names};

fn max_hp_of(model: &mut Ff1Model, id: u32) -> Result<Value, EditError> {
    model.get(&character_field(id, names::MAX_HP))
}

fn next_level_of(model: &mut Ff1Model, id: u32) -> Result<Value, EditError> {
    model.get(&character_field(id, names::NEXT_LEVEL))
}

/// Character 1 of the standard slot with the given status, gain, level text
/// and experience.
fn first(status: &str, gain: u32, level: &str, exp: u32) -> Ff1Model {
    let mut slot = SlotData::standard();
    let character = &mut slot.characters[0];
    character.status_id = Some(status.to_owned());
    character.max_hp_gain = gain;
    level.clone_into(&mut character.level);
    character.exp = exp;
    model(&slot.render())
}

fn party_ids(view: &SaveView, id: u32) -> Vec<String> {
    let party = view.section(Capability::Party).unwrap();
    let entity = party.entities.iter().find(|entity| entity.id == character_field_root(id)).unwrap();
    entity.fields.iter().map(|field| field.id.to_string()).collect()
}

fn character_field_root(id: u32) -> neoffse_core::FieldId {
    neoffse_core::FieldId::new(names::CHARACTER).with_id(id)
}

#[test]
fn max_hp_cases() {
    // (status, gain, shown): 5 reaches the cap, 6 passes it by one, the last
    // gain sits beyond the storage bound.
    let cases = [
        ("1", 0, 35),
        ("2", 833, 863),
        ("3", 12, 45),
        ("4", 1, 31),
        ("5", 966, 999),
        ("6", 975, 999),
        ("1", 4_294_967_295, 999),
    ];
    for (status, gain, shown) in cases {
        let mut model = first(status, gain, "31", 100_100);
        assert_eq!(max_hp_of(&mut model, 1).unwrap(), Value::Int(shown), "status {status}, gain {gain}");
    }
}

#[test]
fn max_hp_ignores_the_current_hp() {
    let mut slot = SlotData::standard();
    slot.characters[0].hp = 1;
    let mut model = model(&slot.render());
    assert_eq!(max_hp_of(&mut model, 1).unwrap(), Value::Int(446));
}

#[test]
fn the_standard_slot_max_hp() {
    let mut model = standard_model();
    let found: Vec<Value> = (1..=4).map(|id| max_hp_of(&mut model, id).unwrap()).collect();
    assert_eq!(found, [446, 452, 466, 469].map(Value::Int));
}

#[test]
fn next_level_cases() {
    // (level text, experience, shown): the stored level does not count.
    let cases = [
        ("1", 0, 14),
        ("1", 13, 1),
        ("1", 14, 28),
        ("1", 15, 27),
        ("5", 0, 14),
        ("0", 5, 9),
        ("69", 820_000, 20_716),
        ("98", 1_449_704, 1),
        ("1", 1_471_454, 1),
    ];
    for (level, exp, shown) in cases {
        let mut model = first("1", 0, level, exp);
        assert_eq!(next_level_of(&mut model, 1).unwrap(), Value::Int(shown), "level {level}, exp {exp}");
    }
}

#[test]
fn next_level_is_absent_without_a_next_row() {
    for (level, exp) in [("1", 1_471_455), ("99", 1_471_455), ("1", 9_999_999), ("99", 9_999_999)] {
        let mut model = first("1", 0, level, exp);
        let field = character_field(1, names::NEXT_LEVEL);
        assert!(matches!(model.get(&field), Err(EditError::UnknownField { .. })), "level {level}, exp {exp}");
        let view = model.view(Language::En).unwrap();
        assert!(!party_ids(&view, 1).contains(&field.to_string()), "level {level}, exp {exp}");
        assert_eq!(party_ids(&view, 1).len(), 42);
    }
    let mut model = first("1", 0, "99", 1_471_454);
    let view = model.view(Language::En).unwrap();
    assert!(party_ids(&view, 1).contains(&character_field(1, names::NEXT_LEVEL).to_string()));
}

#[test]
fn a_negative_experience_leaves_next_level_out() {
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    let path = character_path(0, &[Segment::Doc, Segment::Key("currentExp")]);
    document.set(&path, &Scalar::Int(-1)).unwrap();
    let mut model = Ff1Model::new(document, neoffse_data::tables_for(GAME).unwrap());
    assert!(matches!(next_level_of(&mut model, 1), Err(EditError::UnknownField { .. })));
    let view = model.view(Language::En).unwrap();
    assert_eq!(party_ids(&view, 1).len(), 42);
    assert_eq!(party_ids(&view, 2).len(), 43);
}

#[test]
fn the_standard_slot_next_level() {
    let mut model = standard_model();
    let found: Vec<Value> = (1..=4).map(|id| next_level_of(&mut model, id).unwrap()).collect();
    assert_eq!(found, [3711, 7099, 18_425, 5323].map(Value::Int));
}

#[test]
fn an_unknown_status_leaves_max_hp_out() {
    for status in ["0", "7", "4294967296"] {
        let mut slot = SlotData::standard();
        slot.characters[0].status_id = Some(status.to_owned());
        let mut model = model(&slot.render());
        assert!(matches!(max_hp_of(&mut model, 1), Err(EditError::UnknownField { .. })), "status {status}");
        let view = model.view(Language::En).unwrap();
        let ids = party_ids(&view, 1);
        assert_eq!(ids.len(), 42, "status {status}");
        assert!(!ids.contains(&character_field(1, names::MAX_HP).to_string()), "status {status}");
        assert!(ids.contains(&character_field(1, names::NEXT_LEVEL).to_string()), "status {status}");
    }
}

#[test]
fn a_float_status_fails_the_view() {
    let mut slot = SlotData::standard();
    slot.characters[0].status_id = Some("1.5".to_owned());
    let mut model = model(&slot.render());
    assert!(matches!(max_hp_of(&mut model, 1), Err(EditError::Format { .. })));
    let Err(FormatError::Decode { source }) = model.view(Language::En) else {
        panic!("a float status must fail the view")
    };
    assert!(matches!(source.downcast_ref::<EditError>(), Some(EditError::Format { .. })));
}

#[test]
fn a_missing_status_key_leaves_max_hp_out_and_next_level_in() {
    let mut slot = SlotData::standard();
    slot.characters[0].status_id = None;
    let mut model = model(&slot.render());
    assert!(matches!(max_hp_of(&mut model, 1), Err(EditError::UnknownField { .. })));
    assert_eq!(next_level_of(&mut model, 1).unwrap(), Value::Int(3711));
    let view = model.view(Language::En).unwrap();
    let ids = party_ids(&view, 1);
    assert_eq!(ids.len(), 42);
    assert!(ids.contains(&character_field(1, names::NEXT_LEVEL).to_string()));
}

#[test]
fn a_character_row_that_is_absent_is_missing_row() {
    let mut model = standard_model();
    for name in [names::MAX_HP, names::NEXT_LEVEL] {
        assert!(matches!(model.get(&character_field(9, name)), Err(EditError::MissingRow { .. })), "{name}");
    }
}
