//! Refusals of the raw interface on a spaced slot: a refused call leaves
//! `to_json` byte for byte, because a rewrite before the refusal would
//! compact the spaces. Plus the two row rules of `Remove` and `rows` that a
//! compact slot hides.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{Row, SlotData, character_field, model};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, RowId, Value};
use neoffse_format_pr::document::DocumentError;
use neoffse_format_pr::ff1::names;

const ENFORCE: CapPolicy = CapPolicy::Enforce;
const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;

fn item(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

fn spaced() -> SlotData {
    let mut data = SlotData::standard();
    data.spaced = true;
    data
}

#[test]
fn a_stored_value_outside_the_storage_bound_leaves_a_spaced_text_whole() {
    let mut data = spaced();
    data.characters[0].level = "-5".to_owned();
    let text = data.render();
    let mut model = model(&text);
    let result = model.apply(&Edit::set(character_field(1, names::LEVEL), Value::Int(10)), ENFORCE);
    assert!(matches!(result, Err(EditError::OutOfRange { value: -5, .. })));
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn a_stored_non_integer_leaves_a_spaced_text_whole() {
    let mut data = spaced();
    data.characters[0].level = "31.0".to_owned();
    let text = data.render();
    let mut model = model(&text);
    let result = model.apply(&Edit::set(character_field(1, names::LEVEL), Value::Int(10)), ENFORCE);
    assert!(matches!(result, Err(EditError::Format { .. })));
    assert_eq!(model.document().to_json(), text);
}

#[test]
fn removing_a_non_canonical_row_leaves_a_spaced_text_whole() {
    let rows = [
        r#"{"contentId":4,"count":1,"extra":0}"#,
        r#"{"count":1,"contentId":4}"#,
        r#"{"contentId":4,"count":7.0}"#,
        r#"{"contentId": 4,"count":1}"#,
    ];
    for row in rows {
        let mut data = spaced();
        data.inventory.push(Row::raw(row));
        let text = data.render();
        let mut model = model(&text);
        for policy in [ENFORCE, STORAGE_ONLY] {
            let result = model.apply(&Edit::remove(item(4)), policy);
            assert!(matches!(result, Err(EditError::ReadOnly { .. })), "{row}");
            assert_eq!(model.document().to_json(), text, "{row}");
        }
        assert!(matches!(model.apply_game(&Edit::remove(item(4))), Err(EditError::ReadOnly { .. })), "{row}");
        assert_eq!(model.document().to_json(), text, "{row} through apply_game");
    }
}

#[test]
fn removing_a_row_the_inverse_cannot_store_leaves_a_spaced_text_whole() {
    let mut data = spaced();
    data.inventory.push(Row::new(4, 0));
    let text = data.render();
    let mut model = model(&text);
    for policy in [ENFORCE, STORAGE_ONLY] {
        let result = model.apply(&Edit::remove(item(4)), policy);
        assert!(matches!(result, Err(EditError::OutOfRange { min: 1, .. })));
        assert_eq!(model.document().to_json(), text);
    }
}

#[test]
fn removing_a_row_above_the_cap_works_under_both_policies_and_undoes() {
    // The inverse check runs under StorageOnly: a count of 150 is above
    // MAX_ITEM (99) and inside the storage bound.
    for policy in [ENFORCE, STORAGE_ONLY] {
        let mut data = SlotData::standard();
        data.inventory.push(Row::new(4, 150));
        let text = data.render();
        let mut model = model(&text);
        let inverse = model.apply(&Edit::remove(item(4)), policy).unwrap();
        assert_ne!(model.document().to_json(), text);
        model.apply(&inverse, STORAGE_ONLY).unwrap();
        assert_eq!(model.document().to_json(), text);
    }
}

#[test]
fn a_content_id_above_u32_is_a_format_error_and_never_a_truncated_key() {
    let mut data = SlotData::standard();
    data.inventory.push(Row::raw(r#"{"contentId":4294967303,"count":1}"#));
    let mut model = model(&data.render());
    let Err(EditError::Format { field, source }) = model.rows(&FieldId::new(names::INVENTORY)) else {
        panic!("expected a Format error")
    };
    assert_eq!(field, FieldId::new(names::INVENTORY));
    assert!(matches!(source.downcast_ref::<DocumentError>(), Some(DocumentError::WrongKind { .. })));
    // 4294967303 is 7 modulo 2^32: a truncating read would list the key 7.
    assert!(model.get(&item(7).field(names::COUNT)).is_err());
}
