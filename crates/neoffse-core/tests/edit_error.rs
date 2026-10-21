//! `EditError`: thread safety, messages, sources and the refusals of the model.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::error::Error;

use common::{Model, hp, insert, int, remove, row};
use neoffse_core::{CapPolicy, Capability, ChangeSet, Edit, EditError, FieldId, Game, RowId, TableKind, Value, ValueKind};

fn assert_plain<T: Send + Sync + 'static>() {}

fn assert_error<E: Error>() {}

#[test]
fn edit_types_are_send_sync_and_static() {
    assert_plain::<EditError>();
    assert_plain::<Edit>();
    assert_plain::<ChangeSet>();
    assert_plain::<FieldId>();
    assert_plain::<RowId>();
    assert_plain::<Value>();
    assert_plain::<Game>();
    assert_plain::<Capability>();
    assert_plain::<TableKind>();
    assert_plain::<CapPolicy>();
    assert_error::<EditError>();
}

fn character_hp() -> FieldId {
    hp(1)
}

#[test]
fn display_names_the_field_and_the_numbers() {
    let field = character_hp;
    let seven = RowId::new(FieldId::new("inventory"), 7);
    let cases: Vec<(EditError, Vec<&str>)> = vec![
        (EditError::UnknownField { field: field() }, vec!["character.1.hp"]),
        (EditError::ReadOnly { field: field() }, vec!["character.1.hp"]),
        (
            EditError::WrongType {
                field: field(),
                expected: ValueKind::Int,
                found: ValueKind::Text,
            },
            vec!["character.1.hp", "integer", "text"],
        ),
        (
            EditError::OutOfRange {
                field: field(),
                value: 10_000,
                min: 0,
                max: 9_999,
            },
            vec!["character.1.hp", "10000", "0", "9999"],
        ),
        (
            EditError::AboveCap {
                field: field(),
                value: 1_000,
                cap: 999,
            },
            vec!["character.1.hp", "1000", "999"],
        ),
        (EditError::UnknownId { field: field(), id: 77 }, vec!["character.1.hp", "77"]),
        (EditError::DuplicateRow { row: seven.id().clone() }, vec!["inventory.7"]),
        (EditError::MissingRow { row: seven.id().clone() }, vec!["inventory.7"]),
        (EditError::NotACollection { field: field() }, vec!["character.1.hp"]),
        (
            EditError::PositionOutOfRange {
                collection: FieldId::new("inventory"),
                position: 4,
                len: 3,
            },
            vec!["inventory", "4", "3"],
        ),
    ];
    for (error, parts) in cases {
        let message = error.to_string();
        for part in parts {
            assert!(message.contains(part), "{message:?} lacks {part:?}");
        }
    }
}

#[test]
fn display_does_not_repeat_the_source() {
    let format = EditError::Format {
        field: character_hp(),
        source: Box::new(std::io::Error::other("cause-7d2a")),
    };
    assert!(!format.to_string().contains("cause-7d2a"));
    let source = format.source().unwrap().downcast_ref::<std::io::Error>().unwrap();
    assert_eq!(source.to_string(), "cause-7d2a");

    let inner = EditError::ReadOnly {
        field: FieldId::new("inner-field"),
    };
    let rollback = EditError::UnknownField {
        field: FieldId::new("rollback-field"),
    };
    let failed = EditError::RollbackFailed {
        field: character_hp(),
        source: Box::new(inner),
        rollback: Box::new(rollback),
    };
    let message = failed.to_string();
    assert!(message.contains("character.1.hp"));
    assert!(!message.contains("inner-field"));
    assert!(!message.contains("rollback-field"));
}

#[test]
fn rollback_failed_exposes_the_original_error() {
    let failed = EditError::RollbackFailed {
        field: character_hp(),
        source: Box::new(EditError::ReadOnly { field: FieldId::new("inner") }),
        rollback: Box::new(EditError::UnknownField { field: FieldId::new("other") }),
    };
    let chain_head = failed.source().unwrap();
    assert!(chain_head.downcast_ref::<EditError>().is_none());
    let source = chain_head.downcast_ref::<Box<EditError>>().unwrap();
    assert!(matches!(**source, EditError::ReadOnly { ref field } if field.to_string() == "inner"));
    assert!(chain_head.source().is_none(), "the rollback error is not in the source chain");
    let EditError::RollbackFailed { rollback, .. } = &failed else {
        panic!("expected RollbackFailed");
    };
    assert!(matches!(**rollback, EditError::UnknownField { ref field } if field.to_string() == "other"));
}

#[test]
fn a_refused_edit_leaves_the_model_unchanged() {
    let locked = FieldId::new("character").with_id(1).with_name("locked");
    let name = FieldId::new("character").with_id(1).with_name("name");
    let edits = [
        int(FieldId::new("nowhere"), 1),
        int(locked, 1),
        int(name, 1),
        int(hp(1), 10_000),
        int(hp(1), -1),
        int(hp(1), 1_000),
        int(row(4).field("count"), 5),
        insert(7, None, None),
        Edit::insert(RowId::new(FieldId::new("gil"), 1), None, Vec::new()),
        insert(5, Some(9), None),
        insert(0, None, None),
        remove(4),
    ];
    for edit in edits {
        let mut model = Model::sample();
        let before = model.snapshot();
        assert!(model.apply(&edit, CapPolicy::Enforce).is_err(), "{edit:?}");
        assert_eq!(model.snapshot(), before, "{edit:?}");
    }
}
