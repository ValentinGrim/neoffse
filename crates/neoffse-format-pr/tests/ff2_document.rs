//! `Ff2Document` as a `SaveDocument`: the all-or-none snapshot of `apply_all`
//! on a spaced slot, the FF II twin of the check in `ff1_document`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff2_save::{SlotData, keywords};
use common::pr_files::{one_file, sav, slot_name};
use common::{ENFORCE, row};
use neoffse_core::{ChangeSet, Edit, EditError, FieldId, SaveFormat, Value};
use neoffse_format_pr::container::{Container, decode};
use neoffse_format_pr::ff2::{Ff2Format, names};

#[test]
fn a_failed_apply_all_leaves_a_spaced_document_exact() {
    let mut slot = SlotData::standard();
    slot.spaced = true;
    let text = slot.render();
    let container = Container::game();
    let files = one_file(slot_name(1), sav(&text, &container));
    let format: &dyn SaveFormat = &Ff2Format::new();
    let mut document = format.open(&files).unwrap();
    // The removal rewrites the keyword array compactly; the unknown field then
    // fails, and only the snapshot of `apply_all` gives the layout back.
    let changes = ChangeSet::from_iter([
        Edit::remove(row(names::KEYWORDS, keywords()[0])),
        Edit::set(FieldId::new("nonsense"), Value::Int(1)),
    ]);
    let error = document.apply_all(&changes, ENFORCE).unwrap_err();
    assert!(matches!(error, EditError::UnknownField { .. }), "{error:?}");
    let encoded = document.encode().unwrap();
    let decoded = decode(encoded.get(slot_name(1)).unwrap()).unwrap();
    assert_eq!(decoded.json, text);
    assert_eq!(decoded.container, container);
    assert_eq!(encoded, files);
}
