//! The computed `max_hp` and `max_mp` fields: the stored-gain reading of
//! `docs/formats/pixel-remaster.md#character-growth-in-ff-ii`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_path;
use common::ff2_save::{SlotData, character_field, model, standard_model, tables};
use neoffse_core::{EditError, Value};
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Model, names};

fn derived(model: &mut Ff2Model, id: u32, name: &'static str) -> Result<i64, EditError> {
    match model.get(&character_field(id, name))? {
        Value::Int(number) => Ok(number),
        other => panic!("character {id}, {name}: {other:?}"),
    }
}

fn with_gains(hp: i64, mp: i64) -> Ff2Model {
    let mut slot = SlotData::standard();
    slot.characters[0].max_hp_gain = hp;
    slot.characters[0].max_mp_gain = mp;
    model(&slot.render())
}

#[test]
fn the_maximum_is_the_base_plus_the_gain() {
    let mut model = standard_model();
    // Bases 30, 20, 40, 175 (HP) and 10, 10, 10, 103 (MP), gains of the fixture.
    let hp = [150, 105, 250, 1_575];
    let mp = [24, 19, 16, 183];
    for id in 1..=4 {
        let index = usize::try_from(id).unwrap() - 1;
        assert_eq!(derived(&mut model, id, names::MAX_HP).unwrap(), hp[index], "max_hp {id}");
        assert_eq!(derived(&mut model, id, names::MAX_MP).unwrap(), mp[index], "max_mp {id}");
    }
}

#[test]
fn max_hp_is_held_to_the_cap() {
    let mut model = with_gains(20_000, 0);
    assert_eq!(derived(&mut model, 1, names::MAX_HP).unwrap(), 9_999);
    // The stored gain reads as stored.
    assert_eq!(model.get(&character_field(1, names::MAX_HP_GAIN)).unwrap(), Value::Int(20_000));
    let mut model = with_gains(9_969, 0);
    assert_eq!(derived(&mut model, 1, names::MAX_HP).unwrap(), 9_999);
    let mut model = with_gains(9_968, 0);
    assert_eq!(derived(&mut model, 1, names::MAX_HP).unwrap(), 9_998);
}

#[test]
fn max_mp_is_held_to_the_cap() {
    let mut model = with_gains(0, 5_000);
    assert_eq!(derived(&mut model, 1, names::MAX_MP).unwrap(), 999);
    let mut model = with_gains(0, 989);
    assert_eq!(derived(&mut model, 1, names::MAX_MP).unwrap(), 999);
    let mut model = with_gains(0, 988);
    assert_eq!(derived(&mut model, 1, names::MAX_MP).unwrap(), 998);
}

#[test]
fn a_negative_gain_floors_the_maximum_at_zero() {
    let mut model = with_gains(-50, -11);
    assert_eq!(derived(&mut model, 1, names::MAX_HP).unwrap(), 0);
    assert_eq!(derived(&mut model, 1, names::MAX_MP).unwrap(), 0);
    let mut model = with_gains(-29, -9);
    assert_eq!(derived(&mut model, 1, names::MAX_HP).unwrap(), 1);
    assert_eq!(derived(&mut model, 1, names::MAX_MP).unwrap(), 1);
}

#[test]
fn a_huge_gain_saturates_instead_of_overflowing() {
    let mut model = with_gains(i64::MAX, i64::MAX);
    assert_eq!(derived(&mut model, 1, names::MAX_HP).unwrap(), 9_999);
    assert_eq!(derived(&mut model, 1, names::MAX_MP).unwrap(), 999);
}

fn assert_unknown(result: Result<i64, EditError>, name: &'static str) {
    let field = character_field(1, name);
    assert!(matches!(result, Err(EditError::UnknownField { field: found }) if found == field), "{name}");
}

#[test]
fn a_status_without_a_row_has_no_maximum() {
    for status in ["99", "0", "-1"] {
        let mut slot = SlotData::standard();
        slot.characters[0].status_id = Some(status.to_owned());
        let mut model = model(&slot.render());
        assert_unknown(derived(&mut model, 1, names::MAX_HP), names::MAX_HP);
        assert_unknown(derived(&mut model, 1, names::MAX_MP), names::MAX_MP);
        // The other characters keep their maximum.
        assert_eq!(derived(&mut model, 2, names::MAX_HP).unwrap(), 105);
    }
}

#[test]
fn a_character_without_the_key_has_no_maximum() {
    for key in ["characterStatusId", "addtionalMaxHp", "addtionalMaxMp"] {
        let mut slot = SlotData::standard();
        slot.characters[0].omit = Some(key);
        let mut model = model(&slot.render());
        let (hp, mp) = (derived(&mut model, 1, names::MAX_HP), derived(&mut model, 1, names::MAX_MP));
        match key {
            "addtionalMaxHp" => {
                assert_unknown(hp, names::MAX_HP);
                assert!(mp.is_ok());
            }
            "addtionalMaxMp" => {
                assert!(hp.is_ok());
                assert_unknown(mp, names::MAX_MP);
            }
            _ => {
                assert_unknown(hp, names::MAX_HP);
                assert_unknown(mp, names::MAX_MP);
            }
        }
    }
}

#[test]
fn an_absent_character_is_a_missing_row() {
    let mut model = standard_model();
    assert!(matches!(derived(&mut model, 9, names::MAX_HP), Err(EditError::MissingRow { .. })));
}

#[test]
fn a_gain_of_another_kind_is_a_format_error() {
    let path = character_path(0, &[Segment::Doc, Segment::Key("parameter"), Segment::Doc, Segment::Key("addtionalMaxHp")]);
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    document.set(&path, &Scalar::Text("x".to_owned())).unwrap();
    let mut model = Ff2Model::new(document, tables());
    assert!(matches!(derived(&mut model, 1, names::MAX_HP), Err(EditError::Format { .. })));
}
