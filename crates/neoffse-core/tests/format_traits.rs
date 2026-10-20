//! The `SaveFormat` and `SaveDocument` contracts, driven through trait
//! objects over the fake format.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::error::Error;

use common::fake::{FakeDocument, FakeFormat, add, bytes, character, doc, gil, hp, inventory, locked, open, row};
use neoffse_core::{
    CapPolicy, Capability, ChangeSet, Edit, EditError, FieldId, FormatError, Game, RowId, SaveDocument, SaveFormat, SaveSlot, TableKind, Value,
};

type Verdict = fn(&EditError) -> bool;

fn set(field: FieldId, value: i64) -> Edit {
    Edit::set(field, Value::Int(value))
}

/// A document holding the rows 2, 3 and 5, in that order.
fn three_rows() -> Box<dyn SaveDocument> {
    let mut document = doc();
    document.apply(&add(5, None, 7), CapPolicy::Enforce).unwrap();
    open(&document.encode().unwrap())
}

fn keys_in_view(document: &mut dyn SaveDocument) -> Vec<FieldId> {
    let view = document.view().unwrap();
    let section = view.section(Capability::Inventory).unwrap();
    section.entities.iter().skip(1).map(|entity| entity.id.clone()).collect()
}

fn is_send<T: Send + ?Sized>() {}
fn is_sync<T: Sync + ?Sized>() {}
fn is_error<T: Error + Send + Sync + 'static>() {}

#[test]
fn the_traits_are_dyn_compatible_and_thread_safe() {
    let _format: Box<dyn SaveFormat> = Box::new(FakeFormat);
    let _document: Box<dyn SaveDocument> = doc();
    is_send::<dyn SaveFormat>();
    is_sync::<dyn SaveFormat>();
    is_send::<dyn SaveDocument>();
    is_error::<FormatError>();
    is_error::<EditError>();
}

#[test]
fn a_document_moves_to_a_worker_thread() {
    let mut document = doc();
    let (mut document, inverse) = std::thread::spawn(move || {
        let inverse = document.apply(&set(hp(1), 70), CapPolicy::Enforce).unwrap();
        (document, inverse)
    })
    .join()
    .unwrap();
    document.apply(&inverse, CapPolicy::StorageOnly).unwrap();
    let view = document.view().unwrap();
    assert_eq!(view.find(&hp(1)).unwrap().1.value, Value::Int(50));
}

#[test]
fn open_reports_game_capabilities_and_summary() {
    let mut document = open(&FakeDocument::sample_files());
    assert_eq!(document.game(), Game::Ff1Pr);
    assert_eq!(document.capabilities(), [Capability::Party, Capability::Inventory]);
    let summary = document.summary().unwrap();
    assert_eq!(summary.game, Game::Ff1Pr);
    assert_eq!(summary.party, ["Ada", "Bo"]);
    assert_eq!(summary.slot, Some(SaveSlot::Numbered(1)));
    assert!(summary.play_time.is_some());
}

#[test]
fn every_section_capability_is_reported() {
    let mut document = doc();
    let capabilities = document.capabilities().to_vec();
    let view = document.view().unwrap();
    for section in &view.sections {
        assert!(capabilities.contains(&section.capability));
    }
    for capability in capabilities {
        assert!(view.section(capability).is_some());
    }
}

#[test]
fn apply_updates_the_view_and_returns_the_inverse() {
    let mut document = doc();
    let first = document.view().unwrap();
    let inverse = document.apply(&set(hp(1), 77), CapPolicy::Enforce).unwrap();
    assert_eq!(document.view().unwrap().find(&hp(1)).unwrap().1.value, Value::Int(77));
    assert_eq!(inverse, set(hp(1), 50));
    document.apply(&inverse, CapPolicy::StorageOnly).unwrap();
    assert_eq!(document.view().unwrap(), first);
}

#[test]
fn a_refused_edit_changes_neither_the_view_nor_the_encoding() {
    let cases: Vec<(Edit, CapPolicy, Verdict)> = vec![
        (set(character().with_id(9).with_name("hp"), 1), CapPolicy::Enforce, |e| {
            matches!(e, EditError::UnknownField { .. })
        }),
        (set(locked(), 1), CapPolicy::Enforce, |e| matches!(e, EditError::ReadOnly { .. })),
        (Edit::set(hp(1), Value::Text("x".into())), CapPolicy::Enforce, |e| {
            matches!(e, EditError::WrongType { .. })
        }),
        (set(hp(1), 10_000), CapPolicy::StorageOnly, |e| matches!(e, EditError::OutOfRange { .. })),
        (set(hp(1), 5_000), CapPolicy::Enforce, |e| matches!(e, EditError::AboveCap { .. })),
        (set(row(9).field("count"), 2), CapPolicy::Enforce, |e| matches!(e, EditError::MissingRow { .. })),
        (Edit::remove(row(9)), CapPolicy::Enforce, |e| matches!(e, EditError::MissingRow { .. })),
        (add(2, None, 1), CapPolicy::Enforce, |e| matches!(e, EditError::DuplicateRow { .. })),
        (add(5, Some(9), 1), CapPolicy::Enforce, |e| matches!(e, EditError::PositionOutOfRange { .. })),
        (Edit::insert(RowId::new(gil(), 1), None, vec![]), CapPolicy::Enforce, |e| {
            matches!(e, EditError::NotACollection { .. })
        }),
        (add(100, None, 1), CapPolicy::Enforce, |e| matches!(e, EditError::UnknownId { .. })),
    ];
    let mut document = doc();
    let view = document.view().unwrap();
    let encoding = bytes(&*document);
    for (edit, policy, verdict) in cases {
        let error = document.apply(&edit, policy).unwrap_err();
        assert!(verdict(&error), "{edit:?} gave {error:?}");
        assert_eq!(document.view().unwrap(), view, "{edit:?}");
        assert_eq!(bytes(&*document), encoding, "{edit:?}");
    }
}

#[test]
fn insert_adds_a_row_and_remove_takes_it_back() {
    let mut document = doc();
    let view = document.view().unwrap();
    let encoding = bytes(&*document);
    let inverse = document.apply(&add(5, Some(1), 4), CapPolicy::Enforce).unwrap();
    assert_eq!(inverse, Edit::remove(row(5)));
    let after = document.view().unwrap();
    let inventory_section = after.section(Capability::Inventory).unwrap();
    assert_eq!(inventory_section.entities.get(2).unwrap().id, row(5).id().clone());
    assert_eq!(keys_in_view(&mut *document), [row(2), row(5), row(3)].map(|r| r.id().clone()));
    document.apply(&inverse, CapPolicy::StorageOnly).unwrap();
    assert_eq!(document.view().unwrap(), view);
    assert_eq!(bytes(&*document), encoding);
}

#[test]
fn remove_returns_an_insert_that_restores_the_place() {
    for (key, index, count) in [(2, 0, 3), (3, 1, 1), (5, 2, 7)] {
        let mut document = three_rows();
        let before = bytes(&*document);
        let inverse = document.apply(&Edit::remove(row(key)), CapPolicy::Enforce).unwrap();
        assert_eq!(inverse, Edit::insert(row(key), Some(index), vec![("count", Value::Int(count))]));
        document.apply(&inverse, CapPolicy::StorageOnly).unwrap();
        assert_eq!(bytes(&*document), before, "row {key}");
    }
}

#[test]
fn a_collection_that_forbids_the_kind_refuses_it() {
    let mut document = doc();
    let insert = Edit::insert(RowId::new(character(), 3), None, vec![]);
    let error = document.apply(&insert, CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::ReadOnly { field } if field == character()));
    let error = document.apply(&Edit::remove(RowId::new(character(), 1)), CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::ReadOnly { field } if field == character()));
    let view = document.view().unwrap();
    let meta = view.collection(&character()).unwrap();
    assert!(!meta.insertable && !meta.removable);
}

#[test]
fn the_view_tells_the_ui_what_a_collection_accepts() {
    let view = doc().view().unwrap();
    let meta = view.collection(&inventory()).unwrap();
    assert!(meta.insertable && meta.removable);
    assert_eq!(meta.key.tables, [TableKind::Item]);
    assert_eq!(meta.key.shape, neoffse_core::Shape::Int { min: 1, max: 200 });
    assert_eq!(meta.fields.len(), 1);
}

#[test]
fn the_policy_travels_through_apply() {
    let mut document = doc();
    let error = document.apply(&set(hp(1), 5_000), CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::AboveCap { value: 5_000, cap: 999, .. }));
    document.apply(&set(hp(1), 5_000), CapPolicy::StorageOnly).unwrap();
    let view = document.view().unwrap();
    let (_, field) = view.find(&hp(1)).unwrap();
    assert!(field.meta.exceeds_cap(&field.value));
}

#[test]
fn the_inverse_of_an_expert_edit_applies_under_storage_only() {
    let mut document = doc();
    document.apply(&set(hp(1), 5_000), CapPolicy::StorageOnly).unwrap();
    let expert = bytes(&*document);
    let inverse = document.apply(&set(hp(1), 10), CapPolicy::Enforce).unwrap();
    assert_eq!(inverse, set(hp(1), 5_000));
    let error = document.apply(&inverse, CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::AboveCap { .. }));
    document.apply(&inverse, CapPolicy::StorageOnly).unwrap();
    assert_eq!(bytes(&*document), expert);
}

fn changes(edits: Vec<Edit>) -> ChangeSet {
    edits.into_iter().collect()
}

#[test]
fn apply_all_is_atomic_through_the_trait() {
    let mut document = doc();
    let view = document.view().unwrap();
    let encoding = bytes(&*document);
    let set = changes(vec![set(hp(1), 10), set(gil(), 1_000_000)]);
    let error = document.apply_all(&set, CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::OutOfRange { .. }));
    assert_eq!(document.view().unwrap(), view);
    assert_eq!(bytes(&*document), encoding);
}

#[test]
fn apply_all_rolls_back_rows_and_values() {
    let mut document = doc();
    let encoding = bytes(&*document);
    let set = changes(vec![add(5, Some(0), 2), Edit::remove(row(2)), set(hp(1), 10), set(hp(2), 99_999)]);
    document.apply_all(&set, CapPolicy::Enforce).unwrap_err();
    assert_eq!(bytes(&*document), encoding);
}

#[test]
fn apply_all_rollback_ignores_the_callers_policy() {
    let mut document = doc();
    document.apply(&set(hp(1), 5_000), CapPolicy::StorageOnly).unwrap();
    let encoding = bytes(&*document);
    let set = changes(vec![set(hp(1), 10), set(gil(), 1_000_000)]);
    let error = document.apply_all(&set, CapPolicy::Enforce).unwrap_err();
    assert!(matches!(error, EditError::OutOfRange { .. }), "{error:?}");
    assert_eq!(bytes(&*document), encoding);
}

#[test]
fn apply_all_returns_an_inverse_set() {
    let mut document = doc();
    document.apply(&set(hp(1), 5_000), CapPolicy::StorageOnly).unwrap();
    let before = bytes(&*document);
    let forward = changes(vec![set(hp(1), 10), add(5, None, 4), set(gil(), 500)]);
    let inverse = document.apply_all(&forward, CapPolicy::Enforce).unwrap();
    assert_eq!(inverse.len(), 3);
    assert_ne!(bytes(&*document), before);
    document.apply_all(&inverse, CapPolicy::StorageOnly).unwrap();
    assert_eq!(bytes(&*document), before);
}

#[test]
fn encode_then_open_gives_an_equal_view() {
    let mut document = doc();
    document.apply(&add(5, Some(0), 4), CapPolicy::Enforce).unwrap();
    document.apply(&set(hp(2), 12), CapPolicy::Enforce).unwrap();
    let mut reopened = open(&document.encode().unwrap());
    assert_eq!(reopened.view().unwrap(), document.view().unwrap());
    assert_eq!(reopened.summary().unwrap(), document.summary().unwrap());
    assert_eq!(keys_in_view(&mut *reopened), [row(5), row(2), row(3)].map(|r| r.id().clone()));
}
