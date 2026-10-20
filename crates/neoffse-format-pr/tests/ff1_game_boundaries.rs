//! Two boundaries of the game interface: the rollback of a two-step plan on
//! a spaced text, and the equipment keys that count as slots.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{Equip, Row, SlotData, character_field, model};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, RowId, Value};
use neoffse_format_pr::ff1::names;

fn item(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

fn slot(id: u32, number: u32) -> FieldId {
    character_field(id, names::EQUIPMENT).with_id(number)
}

/// The unequip of slot 1 is a two-step plan: the slot `contentId`, then the
/// slot `count` copied from the placeholder row. A stored placeholder `count`
/// of -5 fails the second step on the storage bound after the first one
/// wrote.
#[test]
fn a_two_step_plan_that_fails_at_step_two_restores_the_spaced_text() {
    let mut data = SlotData::standard();
    data.spaced = true;
    data.inventory[0] = Row::raw(r#"{"contentId":62,"count":-5}"#);
    let text = data.render();
    let mut model = model(&text);
    let edit = Edit::set(slot(1, 1), Value::Int(62));

    let error = model.apply_game(&edit).unwrap_err();
    assert!(matches!(error, EditError::OutOfRange { value: -5, min: 0, .. }), "{error}");
    assert_eq!(model.document().to_json(), text);

    // The raw interface wrote step 1 before the failure: the snapshot is what
    // restores the spaced levels.
    let mut raw = model.clone();
    raw.apply(&edit, CapPolicy::Enforce).unwrap();
    assert_ne!(raw.document().to_json(), text);
}

/// Only the keys 1 to 5 of `equipmentList` are slots. A worn id under any
/// other key is no wearer.
#[test]
fn an_equipment_key_outside_the_slots_is_no_wearer() {
    for key in [0, 6, 7] {
        let mut data = SlotData::standard();
        data.characters[0].equipment.push(Equip {
            key,
            content_id: "100".to_owned(),
            count: Some("1".to_owned()),
        });
        let mut model = model(&data.render());
        assert!(!model.row_locked(&item(100)).unwrap(), "key {key}");
        assert!(model.apply_game(&Edit::remove(item(100))).is_ok(), "key {key}");
    }
    let mut data = SlotData::standard();
    data.characters[0].equipment[4].content_id = "100".to_owned();
    assert!(model(&data.render()).row_locked(&item(100)).unwrap());
}
