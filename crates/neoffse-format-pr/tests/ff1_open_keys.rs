//! The shape gate of `Ff1Format::open_document` on row keys and on `userData`:
//! a repeated `id` or `contentId` is not a slot, and neither is a `userData`
//! string that holds no JSON object.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_format::{one_file, sav, slot_name};
use common::ff1_save::{Row, SlotData};
use neoffse_core::{EditError, FieldId, FormatError, RowId};
use neoffse_format_pr::container::Container;
use neoffse_format_pr::document::{Document, Segment};
use neoffse_format_pr::ff1::{Ff1Document, Ff1Format, OpenError, names};

fn open_text(text: &str) -> Result<Ff1Document, FormatError> {
    let files = one_file(slot_name(1), sav(text, &Container::game()));
    Ff1Format::new().open_document(&files)
}

/// The `NotASlot` source of a refused open, or a panic naming what came instead.
fn not_a_slot(result: Result<Ff1Document, FormatError>) -> Box<dyn std::error::Error + Send + Sync> {
    let Err(FormatError::Decode { source }) = result else {
        panic!("expected a Decode error")
    };
    match *source.downcast::<OpenError>().expect("the source is an OpenError") {
        OpenError::NotASlot { source } => source,
        other => panic!("expected NotASlot, got {other:?}"),
    }
}

fn row_id(collection: &'static str, key: u32) -> FieldId {
    RowId::new(FieldId::new(collection), key).id().clone()
}

#[test]
fn a_repeated_inventory_content_id_is_not_a_slot() {
    let mut data = SlotData::standard();
    data.inventory.push(Row::new(89, 3));
    let source = not_a_slot(open_text(&data.render()));
    let error = source.downcast_ref::<EditError>().expect("the source is an EditError");
    assert!(matches!(error, EditError::DuplicateRow { row } if *row == row_id(names::INVENTORY, 89)));
}

#[test]
fn a_repeated_key_item_is_not_a_slot() {
    let mut data = SlotData::standard();
    data.key_items.push(Row::new(46, 1));
    let source = not_a_slot(open_text(&data.render()));
    let error = source.downcast_ref::<EditError>().expect("the source is an EditError");
    assert!(matches!(error, EditError::DuplicateRow { row } if *row == row_id(names::KEY_ITEMS, 46)));
}

#[test]
fn a_repeated_character_id_is_not_a_slot() {
    let mut data = SlotData::standard();
    data.characters.push(data.characters[0].clone());
    let source = not_a_slot(open_text(&data.render()));
    let error = source.downcast_ref::<EditError>().expect("the source is an EditError");
    assert!(matches!(error, EditError::DuplicateRow { row } if *row == row_id(names::CHARACTER, 1)));
}

#[test]
fn the_first_second_occurrence_in_document_order_is_named() {
    let mut data = SlotData::standard();
    data.inventory.push(Row::new(100, 1));
    data.inventory.push(Row::new(89, 1));
    let source = not_a_slot(open_text(&data.render()));
    let error = source.downcast_ref::<EditError>().expect("the source is an EditError");
    assert!(matches!(error, EditError::DuplicateRow { row } if *row == row_id(names::INVENTORY, 100)));
}

#[test]
fn distinct_keys_across_the_lists_open() {
    // 45 sits in key_items only, 89 in inventory only: the same number in two
    // lists is no repeat.
    let mut data = SlotData::standard();
    data.key_items.push(Row::new(89, 1));
    assert!(open_text(&data.render()).is_ok());
}

#[test]
fn a_user_data_string_that_is_no_object_is_not_a_slot() {
    for token in [r#""[]""#, r#""1""#] {
        let mut document = Document::parse(&SlotData::standard().render()).unwrap();
        document.set_raw(&[Segment::Key("userData")], token).unwrap();
        let text = document.to_json();
        let files = one_file(slot_name(1), sav(&text, &Container::game()));
        let result = Ff1Format::new().open_document(&files);
        assert!(
            matches!(result, Err(FormatError::Decode { ref source }) if matches!(source.downcast_ref::<OpenError>(), Some(OpenError::NotASlot { .. }))),
            "userData {token}"
        );
    }
}

#[test]
fn a_model_built_by_hand_accepts_a_repeated_key_and_reads_the_first_row() {
    let mut data = SlotData::standard();
    data.inventory.push(Row::new(89, 7));
    let mut model = common::ff1_save::model(&data.render());
    let keys = model.rows(&FieldId::new(names::INVENTORY)).unwrap();
    assert_eq!(keys.iter().filter(|key| **key == 89).count(), 2);
    let first = RowId::new(FieldId::new(names::INVENTORY), 89).field(names::COUNT);
    assert_eq!(model.get(&first).unwrap(), neoffse_core::Value::Int(1));
}
