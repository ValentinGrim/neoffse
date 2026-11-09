//! The shape gate of `Ff2Format::open_document` on the keyword list: a
//! repeated keyword is not a slot, because a `Remove` of it returns an
//! `Insert` that fails (`docs/formats/pixel-remaster.md#keywords-in-ff-ii`),
//! and neither is a slot with no keyword list or a keyword that is no integer.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff2_save::{SlotData, keywords};
use common::pr_files::{one_file, open_error, sav, slot_name, source_of};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, RowId, SaveDocument};
use neoffse_format_pr::OpenError;
use neoffse_format_pr::container::Container;
use neoffse_format_pr::document::{Document, DocumentError, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Document, Ff2Format, names};

fn open_data(data: &SlotData) -> Result<Ff2Document, neoffse_core::FormatError> {
    let files = one_file(slot_name(1), sav(&data.render(), &Container::game()));
    Ff2Format::new().open_document(&files)
}

fn keyword_row(key: u32) -> RowId {
    RowId::new(FieldId::new(names::KEYWORDS), key)
}

#[test]
fn a_repeated_keyword_is_not_a_slot() {
    let mut data = SlotData::standard();
    let repeated = keywords()[0];
    data.keywords.push(repeated);
    let source = source_of(open_data(&data));
    let Some(OpenError::NotASlot { source }) = source.downcast_ref::<OpenError>() else {
        panic!("expected NotASlot")
    };
    let error = source.downcast_ref::<EditError>().expect("the source is an EditError");
    assert!(
        matches!(error, EditError::DuplicateRow { row } if row == keyword_row(repeated).id()),
        "{error:?}"
    );
}

#[test]
fn the_same_slot_without_the_repeat_opens() {
    assert!(open_data(&SlotData::standard()).is_ok());
}

#[test]
fn a_keyword_zero_opens_and_its_raw_removal_is_refused_with_the_text_unchanged() {
    let mut data = SlotData::standard();
    data.keywords.push(0);
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        let mut document = open_data(&data).unwrap();
        let before = document.encode().unwrap();
        let error = document.apply(&Edit::remove(keyword_row(0)), policy).unwrap_err();
        // The meta bounds a keyword at 1: the removal would return an
        // `Insert` of 0 that cannot apply, so it stays.
        assert!(matches!(error, EditError::OutOfRange { value: 0, min: 1, .. }), "{policy:?}: {error:?}");
        assert_eq!(document.encode().unwrap(), before, "{policy:?}");
    }
}

/// The document error under the `NotASlot` error that opening `text` gives,
/// once it names the keyword list.
fn keyword_list_failure(text: &str) -> DocumentError {
    let files = one_file(slot_name(1), sav(text, &Container::game()));
    let OpenError::NotASlot { source } = open_error(Ff2Format::new().open_document(&files)) else {
        panic!("expected NotASlot")
    };
    let EditError::Format { field, source } = *source.downcast::<EditError>().expect("the source is an EditError") else {
        panic!("expected a Format error")
    };
    assert_eq!(field, FieldId::new(names::KEYWORDS));
    *source.downcast::<DocumentError>().expect("the source is a DocumentError")
}

#[test]
fn a_slot_without_the_keyword_list_is_not_a_slot() {
    let text = SlotData::standard().render();
    // The key renamed reads as absent, as in `ff2_open`.
    let renamed = text.replacen("\\\"ownedKeyWaordList\\\"", "\\\"ownedKeyWaordListX\\\"", 1);
    assert_ne!(renamed, text, "the rename found the key");
    let error = keyword_list_failure(&renamed);
    assert!(
        matches!(&error, DocumentError::MissingKey { path, key } if path == "userData" && key == "ownedKeyWaordList"),
        "{error:?}"
    );
}

#[test]
fn a_keyword_that_is_no_integer_is_not_a_slot() {
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    let element = [
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("ownedKeyWaordList"),
        Segment::Doc,
        Segment::Key("target"),
        Segment::Index(1),
    ];
    document.set(&element, &Scalar::Text("word".to_owned())).unwrap();
    let error = keyword_list_failure(&document.to_json());
    assert!(
        matches!(&error, DocumentError::WrongKind { path, expected: "integer" } if path == "userData.ownedKeyWaordList.target[1]"),
        "{error:?}"
    );
}
