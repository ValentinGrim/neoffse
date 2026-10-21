//! `notice_of` and `problem_notice`: an edit error becomes a problem kind and
//! decimal numbers, never the error's text.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::core::{EditError, FieldId, RowId, ValueKind};
use neoffse_app::editor::{notice_of, problem_notice};
use neoffse_app::{EditNotice, EditProblem};

const LABEL: &str = "Aria level";

fn field() -> FieldId {
    FieldId::new("character").with_id(1).with_name("level")
}

fn row() -> FieldId {
    RowId::new(FieldId::new("inventory"), 100).id().clone()
}

fn strings(notice: &EditNotice) -> [&str; 5] {
    [
        notice.label.as_str(),
        notice.value.as_str(),
        notice.min.as_str(),
        notice.max.as_str(),
        notice.cap.as_str(),
    ]
}

/// The problem and the five strings, in the order label, value, min, max, cap.
fn read(error: &EditError) -> (EditProblem, [String; 5]) {
    let notice = notice_of(error, LABEL);
    (notice.problem, strings(&notice).map(str::to_owned))
}

fn expect(error: &EditError, problem: EditProblem, value: &str, min: &str, max: &str, cap: &str) {
    let (found, texts) = read(error);
    assert_eq!(found, problem);
    assert_eq!(texts, [LABEL, value, min, max, cap].map(str::to_owned));
}

fn every_error() -> Vec<EditError> {
    vec![
        EditError::ReadOnly { field: field() },
        EditError::WrongType {
            field: field(),
            expected: ValueKind::Int,
            found: ValueKind::Text,
        },
        EditError::UnknownField { field: field() },
        EditError::NotACollection { field: field() },
        EditError::PositionOutOfRange {
            collection: field(),
            position: 9,
            len: 2,
        },
        EditError::OutOfRange {
            field: field(),
            value: 5,
            min: 2,
            max: 9,
        },
        EditError::AboveCap {
            field: field(),
            value: 100,
            cap: 99,
        },
        EditError::UnknownId { field: field(), id: 13 },
        EditError::DuplicateRow { row: row() },
        EditError::MissingRow { row: row() },
        EditError::RollbackFailed {
            field: field(),
            source: Box::new(EditError::ReadOnly { field: field() }),
            rollback: Box::new(EditError::MissingRow { row: row() }),
        },
        EditError::Format {
            field: field(),
            source: Box::new(std::io::Error::other("disk detail")),
        },
    ]
}

#[test]
fn read_only_maps_to_read_only() {
    expect(&EditError::ReadOnly { field: field() }, EditProblem::ReadOnly, "", "", "", "");
}

#[test]
fn structural_errors_map_to_not_editable() {
    let cases = [
        EditError::WrongType {
            field: field(),
            expected: ValueKind::Int,
            found: ValueKind::Text,
        },
        EditError::UnknownField { field: field() },
        EditError::NotACollection { field: field() },
        EditError::PositionOutOfRange {
            collection: field(),
            position: 9,
            len: 2,
        },
    ];
    for error in &cases {
        expect(error, EditProblem::NotEditable, "", "", "", "");
    }
}

#[test]
fn out_of_range_carries_the_errors_own_numbers() {
    // A game floor, not the storage bound: the notice prints it as given.
    let error = EditError::OutOfRange {
        field: field(),
        value: -4,
        min: 2,
        max: 2_147_483_647,
    };
    expect(&error, EditProblem::OutOfRange, "-4", "2", "2147483647", "");
}

#[test]
fn above_cap_carries_value_and_cap() {
    let error = EditError::AboveCap {
        field: field(),
        value: 100,
        cap: 99,
    };
    expect(&error, EditProblem::AboveCap, "100", "", "", "99");
}

#[test]
fn unknown_id_carries_the_id_as_value() {
    let error = EditError::UnknownId { field: field(), id: 13 };
    expect(&error, EditProblem::UnknownId, "13", "", "", "");
    let negative = EditError::UnknownId { field: field(), id: i64::MIN };
    expect(&negative, EditProblem::UnknownId, &i64::MIN.to_string(), "", "", "");
}

#[test]
fn row_errors_map_to_their_problems() {
    expect(&EditError::DuplicateRow { row: row() }, EditProblem::DuplicateRow, "", "", "", "");
    expect(&EditError::MissingRow { row: row() }, EditProblem::MissingRow, "", "", "", "");
}

#[test]
fn a_failed_rollback_maps_to_reloaded() {
    let error = EditError::RollbackFailed {
        field: field(),
        source: Box::new(EditError::ReadOnly { field: field() }),
        rollback: Box::new(EditError::MissingRow { row: row() }),
    };
    expect(&error, EditProblem::Reloaded, "", "", "", "");
}

#[test]
fn a_format_error_maps_to_internal() {
    let error = EditError::Format {
        field: field(),
        source: Box::new(std::io::Error::other("disk detail")),
    };
    expect(&error, EditProblem::Internal, "", "", "", "");
}

#[test]
fn a_notice_never_carries_the_error_text() {
    let path = field().to_string();
    assert!(path.contains("character"), "the path renders as text");
    for error in every_error() {
        let notice = notice_of(&error, LABEL);
        let display = error.to_string();
        for text in strings(&notice) {
            for forbidden in [path.as_str(), "character", "inventory", "disk detail", display.as_str()] {
                assert!(!text.contains(forbidden), "a notice string holds error text for {:?}", notice.problem);
            }
        }
    }
}

#[test]
fn problem_notice_has_no_numbers() {
    let notice = problem_notice(EditProblem::NotANumber, "Gil");
    assert_eq!(notice.problem, EditProblem::NotANumber);
    assert_eq!(notice.label.as_str(), "Gil");
    assert_eq!([notice.value.as_str(), notice.min.as_str(), notice.max.as_str(), notice.cap.as_str()], [""; 4]);
}
