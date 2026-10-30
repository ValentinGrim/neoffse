//! `Ff1Document` as a `SaveDocument`: encode, verify and the wiring of
//! `apply` and `apply_all`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_format::{one_file, open, sav, slot_name, standard_files};
use common::ff1_save::SlotData;
use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FieldId, FormatError, RowId, SaveDocument, SaveFiles, Value};
use neoffse_format_pr::container::{CodecError, Container, MAX_PAYLOAD_LEN, decode};
use neoffse_format_pr::ff1::{Ff1Document, Ff1Format, names};

const ENFORCE: CapPolicy = CapPolicy::Enforce;
const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;

fn gil() -> FieldId {
    FieldId::new(names::GIL)
}

fn set_gil(value: i64) -> Edit {
    Edit::set(gil(), Value::Int(value))
}

fn containers() -> [Container; 3] {
    [
        Container::game(),
        Container { bom: true, trailer: vec![] },
        Container {
            bom: false,
            trailer: b" \r\n\t".to_vec(),
        },
    ]
}

fn original() -> String {
    SlotData::standard().render()
}

fn decoded_text(files: &SaveFiles, name: &str) -> String {
    decode(files.get(name).unwrap()).unwrap().json
}

fn opened(text: &str) -> Ff1Document {
    open(&one_file(slot_name(1), sav(text, &Container::game())))
}

#[test]
fn an_unedited_document_round_trips() {
    for container in containers() {
        let files = one_file(slot_name(1), sav(&original(), &container));
        let encoded = open(&files).encode().unwrap();
        assert_eq!(encoded.len(), 1);
        let decoded = decode(encoded.get(slot_name(1)).unwrap()).unwrap();
        assert_eq!(decoded.json, original());
        assert_eq!(decoded.container, container);
    }
}

#[test]
fn encode_changes_nothing() {
    let document = open(&standard_files(&Container::game()));
    assert_eq!(document.encode().unwrap(), document.encode().unwrap());
}

#[test]
fn apply_reaches_encode() {
    let mut document = opened(&original());
    document.apply(&set_gil(777), ENFORCE).unwrap();
    let encoded = document.encode().unwrap();
    let text = decoded_text(&encoded, slot_name(1));
    assert_ne!(text, original());
    let mut fresh = Ff1Format::new().open_document(&encoded).unwrap();
    let view = fresh.view().unwrap();
    let (_, field) = view.find(&gil()).unwrap();
    assert_eq!(field.value, Value::Int(777));
}

#[test]
fn apply_returns_the_inverse() {
    let mut document = opened(&original());
    let inverse = document.apply(&set_gil(5), ENFORCE).unwrap();
    assert_eq!(inverse, set_gil(123_456));
    document.apply(&inverse, STORAGE_ONLY).unwrap();
    assert_eq!(decoded_text(&document.encode().unwrap(), slot_name(1)), original());
}

#[test]
fn the_policy_reaches_the_model() {
    let mut document = opened(&original());
    let gil = set_gil(1_000_000);
    assert!(matches!(document.apply(&gil, ENFORCE), Err(EditError::AboveCap { .. })));
    assert!(document.apply(&gil, STORAGE_ONLY).is_ok());
}

#[test]
fn edit_errors_pass_through_unchanged() {
    let mut document = opened(&original());
    let unknown = Edit::set(FieldId::new("nonsense"), Value::Int(1));
    assert!(matches!(document.apply(&unknown, ENFORCE), Err(EditError::UnknownField { .. })));
    let character = FieldId::new(names::CHARACTER).with_id(9).with_name(names::LEVEL);
    let missing = Edit::set(character, Value::Int(1));
    let row = RowId::new(FieldId::new(names::CHARACTER), 9);
    assert!(matches!(document.apply(&missing, ENFORCE), Err(EditError::MissingRow { row: found }) if &found == row.id()));
    assert!(matches!(document.apply(&set_gil(-1), ENFORCE), Err(EditError::OutOfRange { .. })));
}

#[test]
fn a_failed_set_leaves_a_spaced_document_exact() {
    let mut slot = SlotData::standard();
    slot.spaced = true;
    let text = slot.render();
    let mut document = opened(&text);
    let changes = ChangeSet::from_iter([set_gil(5), Edit::set(FieldId::new("nonsense"), Value::Int(1))]);
    assert!(matches!(document.apply_all(&changes, ENFORCE), Err(EditError::UnknownField { .. })));
    assert_eq!(decoded_text(&document.encode().unwrap(), slot_name(1)), text);
}

#[test]
fn a_good_set_returns_an_inverse_set_that_restores() {
    let mut document = opened(&original());
    let level = FieldId::new(names::CHARACTER).with_id(2).with_name(names::LEVEL);
    let changes = ChangeSet::from_iter([set_gil(5), Edit::set(level, Value::Int(60))]);
    let inverse = document.apply_all(&changes, ENFORCE).unwrap();
    assert_ne!(decoded_text(&document.encode().unwrap(), slot_name(1)), original());
    document.apply_all(&inverse, STORAGE_ONLY).unwrap();
    assert_eq!(decoded_text(&document.encode().unwrap(), slot_name(1)), original());
}

#[test]
fn a_single_failed_edit_changes_nothing() {
    let mut document = opened(&original());
    let changes = ChangeSet::from_iter([set_gil(-1)]);
    assert!(document.apply_all(&changes, ENFORCE).is_err());
    assert_eq!(decoded_text(&document.encode().unwrap(), slot_name(1)), original());
}

#[test]
fn verify_accepts_its_own_encoding() {
    let mut document = opened(&original());
    document.apply(&set_gil(9), ENFORCE).unwrap();
    document.verify(&document.encode().unwrap()).unwrap();
}

#[test]
fn verify_accepts_the_original_file() {
    for container in containers() {
        let files = one_file(slot_name(1), sav(&original(), &container));
        open(&files).verify(&files).unwrap();
    }
}

#[test]
fn verify_refuses_a_changed_document() {
    let mut document = opened(&original());
    let files = document.encode().unwrap();
    document.apply(&set_gil(1), ENFORCE).unwrap();
    assert!(matches!(document.verify(&files), Err(FormatError::VerifyMismatch { name }) if name == slot_name(1)));
}

#[test]
fn verify_refuses_a_one_digit_difference() {
    let document = opened(&original());
    let other = original().replacen("123456", "123457", 1);
    assert_ne!(other, original());
    let files = one_file(slot_name(1), sav(&other, &Container::game()));
    assert!(matches!(document.verify(&files), Err(FormatError::VerifyMismatch { .. })));
}

#[test]
fn verify_refuses_another_container() {
    let document = opened(&original());
    let other = Container {
        bom: false,
        trailer: b"\r\n".to_vec(),
    };
    let files = one_file(slot_name(1), sav(&original(), &other));
    assert!(matches!(document.verify(&files), Err(FormatError::VerifyMismatch { .. })));
}

#[test]
fn verify_names_a_missing_file() {
    let document = opened(&original());
    let bytes = sav(&original(), &Container::game());
    for files in [SaveFiles::new(), one_file(slot_name(2), bytes)] {
        assert!(matches!(document.verify(&files), Err(FormatError::MissingFile { name }) if name == slot_name(1)));
    }
}

#[test]
fn verify_reports_unreadable_bytes_as_decode() {
    let document = opened(&original());
    let files = one_file(slot_name(1), b"not a save \xff".to_vec());
    let Err(FormatError::Decode { source }) = document.verify(&files) else {
        panic!("expected Decode")
    };
    assert!(source.downcast::<CodecError>().is_ok());
}

#[test]
fn verify_ignores_other_files() {
    let document = opened(&original());
    let mut files = document.encode().unwrap();
    files.push(slot_name(2), b"anything".to_vec()).unwrap();
    document.verify(&files).unwrap();
}

/// `container::encode` refuses a text above `MAX_PAYLOAD_LEN`, so a document
/// that large is refused before any file exists. After the inverse edit, the
/// document encodes again.
#[test]
fn an_oversized_document_is_refused_by_encode() {
    let mut document = opened(&original());
    let name = FieldId::new(names::CHARACTER).with_id(1).with_name(names::NAME);
    let huge = Edit::set(name, Value::Text("x".repeat(17 * 1024 * 1024)));
    let inverse = document.apply(&huge, ENFORCE).unwrap();
    let Err(FormatError::Encode { source }) = document.encode() else {
        panic!("expected Encode")
    };
    assert!(matches!(
        source.downcast_ref::<CodecError>(),
        Some(CodecError::PayloadTooLarge { limit: MAX_PAYLOAD_LEN })
    ));
    document.apply(&inverse, STORAGE_ONLY).unwrap();
    assert_eq!(decoded_text(&document.encode().unwrap(), slot_name(1)), original());
    assert!(document.summary().is_ok());
}
