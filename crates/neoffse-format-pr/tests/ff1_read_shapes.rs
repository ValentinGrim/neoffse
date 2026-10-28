//! Read rules for stored values of the wrong shape, and the field list of a
//! character.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_path;
use common::ff1_save::{SlotData, character_field};
use neoffse_core::{EditError, FieldId};
use neoffse_format_pr::document::{Document, DocumentError, Segment};
use neoffse_format_pr::ff1::{Ff1Model, GAME, names};

/// The standard slot with the raw JSON `raw` at `key` of the first character.
fn model_with(key: &'static str, raw: &str) -> Ff1Model {
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    document.set_raw(&character_path(0, &[Segment::Doc, Segment::Key(key)]), raw).unwrap();
    Ff1Model::new(document, neoffse_data::tables_for(GAME).unwrap())
}

/// Whether `error` is a `Format` error whose source is a `WrongKind`.
fn is_wrong_kind(error: &EditError) -> bool {
    let EditError::Format { source, .. } = error else {
        return false;
    };
    matches!(source.downcast_ref::<DocumentError>(), Some(DocumentError::WrongKind { .. }))
}

#[test]
fn a_character_id_above_u32_is_a_format_error() {
    let mut model = model_with("id", "4294967297");
    let result = model.rows(&FieldId::new(names::CHARACTER));
    assert!(result.as_ref().is_err_and(is_wrong_kind), "{result:?}");
}

#[test]
fn a_character_name_that_is_a_number_is_a_wrong_kind() {
    let mut model = model_with("name", "5");
    let result = model.get(&character_field(1, names::NAME));
    assert!(result.as_ref().is_err_and(is_wrong_kind), "{result:?}");
}

#[test]
fn a_character_has_41_fields_in_display_order() {
    let fields = names::character_fields(1);
    assert_eq!(fields.len(), 41);
    assert_eq!(fields[0], character_field(1, names::NAME));
    let model = common::ff1_save::standard_model();
    let sample = [
        (character_field(1, names::NAME), "Name"),
        (character_field(1, names::MAX_HP_GAIN), "Max HP gain"),
        (character_field(1, names::CHARGES).with_id(3), "Charges, level 3"),
        (character_field(1, names::CHARGES_MAX_GAIN).with_id(8), "Max charges gain, level 8"),
        (character_field(1, names::EQUIPMENT).with_id(2), "Left hand"),
    ];
    for (field, label) in sample {
        assert!(fields.contains(&field), "{field}");
        assert_eq!(model.field_meta(&field).unwrap().label, label, "{field}");
    }
}
