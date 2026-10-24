//! `review_rows`, `stamp_of` and the three save notices, over hand-built
//! values: no session, no window.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::core::{Capability, FieldId};
use neoffse::{Change, ChangeKind};
use neoffse_app::editor::{reload_failed_notice, review_rows, save_failed_notice, saved_notice, stamp_of};
use neoffse_app::{EditNotice, EditProblem, ReviewKind, ReviewRow};

fn field(capability: Capability, entity: &'static str, label: &str, name: &'static str, before: &str, after: &str) -> Change {
    let id = FieldId::new(entity);
    Change::new(
        capability,
        id.clone(),
        label,
        ChangeKind::Field {
            field: id.with_name(name),
            label: name.to_owned(),
            before: before.to_owned(),
            after: after.to_owned(),
        },
    )
}

fn kinds(rows: &[ReviewRow]) -> Vec<ReviewKind> {
    rows.iter().map(|row| row.kind).collect()
}

fn heading(entity: &str) -> ReviewRow {
    ReviewRow {
        kind: ReviewKind::Heading,
        entity: entity.into(),
        ..ReviewRow::default()
    }
}

#[test]
fn an_empty_slice_gives_no_row() {
    assert_eq!(review_rows(&[]).len(), 0);
}

#[test]
fn one_field_change_gives_a_heading_and_a_field_row() {
    let rows = review_rows(&[field(Capability::Party, "character", "Aria", "level", "31", "34")]);
    assert_eq!(
        rows,
        vec![
            heading("Aria"),
            ReviewRow {
                kind: ReviewKind::Field,
                entity: "Aria".into(),
                label: "level".into(),
                before: "31".into(),
                after: "34".into(),
            },
        ]
    );
}

#[test]
fn two_fields_of_one_entity_share_a_heading() {
    let rows = review_rows(&[
        field(Capability::Party, "character", "Aria", "level", "31", "34"),
        field(Capability::Party, "character", "Aria", "job", "Knight", "Ninja"),
    ]);
    assert_eq!(kinds(&rows), [ReviewKind::Heading, ReviewKind::Field, ReviewKind::Field]);
}

#[test]
fn two_entities_give_two_headings() {
    let rows = review_rows(&[
        field(Capability::Party, "character", "Aria", "level", "31", "34"),
        field(Capability::Party, "other", "Bram", "level", "5", "6"),
    ]);
    assert_eq!(kinds(&rows), [ReviewKind::Heading, ReviewKind::Field, ReviewKind::Heading, ReviewKind::Field]);
    assert_eq!(rows[2].entity.as_str(), "Bram");
}

#[test]
fn added_and_removed_rows_name_the_entity() {
    let rows = review_rows(&[
        Change::new(Capability::Inventory, FieldId::new("inventory"), "X-Potion", ChangeKind::RowAdded),
        Change::new(Capability::Inventory, FieldId::new("other"), "Phoenix Down", ChangeKind::RowRemoved),
    ]);
    assert_eq!(kinds(&rows), [ReviewKind::Heading, ReviewKind::Added, ReviewKind::Heading, ReviewKind::Removed]);
    assert_eq!(rows[1].entity.as_str(), "X-Potion");
    assert_eq!(rows[3].entity.as_str(), "Phoenix Down");
    for row in [&rows[1], &rows[3]] {
        assert!(row.label.is_empty() && row.before.is_empty() && row.after.is_empty());
    }
}

#[test]
fn the_same_entity_in_two_separate_runs_gets_two_headings() {
    let rows = review_rows(&[
        field(Capability::Party, "character", "Aria", "level", "31", "34"),
        field(Capability::Party, "other", "Bram", "level", "5", "6"),
        field(Capability::Party, "character", "Aria", "job", "Knight", "Ninja"),
    ]);
    let headings = rows.iter().filter(|row| row.kind == ReviewKind::Heading).count();
    assert_eq!(headings, 3);
}

#[test]
fn the_same_entity_id_under_two_capabilities_gets_two_headings() {
    let rows = review_rows(&[
        field(Capability::Party, "character", "Aria", "level", "31", "34"),
        field(Capability::Inventory, "character", "Aria", "count", "1", "2"),
    ]);
    assert_eq!(kinds(&rows), [ReviewKind::Heading, ReviewKind::Field, ReviewKind::Heading, ReviewKind::Field]);
}

#[test]
fn a_stamp_in_utc_names_the_zone() {
    assert_eq!(stamp_of("2026-10-22T14:05:09Z").as_deref(), Some("2026-10-22 14:05 UTC"));
}

#[test]
fn a_stamp_with_an_offset_keeps_it_as_written() {
    assert_eq!(stamp_of("2026-10-22T16:05:09+02:00").as_deref(), Some("2026-10-22 16:05 +02:00"));
    assert_eq!(stamp_of("2026-10-22T09:05:09-05:30").as_deref(), Some("2026-10-22 09:05 -05:30"));
}

#[test]
fn a_text_the_parser_refuses_gives_none() {
    for text in [
        "",
        "+00:00",
        "2026-13-01T00:00:00Z",
        "2026-10-22T14:05:09z",
        "2026-10-22T14:05:09.5Z",
        "2026-10-22T14:05:09Z trailing",
        "2026-10-22 14:05:09Z",
    ] {
        assert_eq!(stamp_of(text), None, "{text:?}");
    }
}

fn blank_but(notice: &EditNotice, value: &str) {
    assert_eq!(notice.value.as_str(), value);
    for text in [&notice.label, &notice.min, &notice.max, &notice.cap] {
        assert!(text.is_empty());
    }
}

#[test]
fn the_saved_notice_carries_the_stamp() {
    let notice = saved_notice(Some("2026-10-22 14:05 UTC"));
    assert_eq!(notice.problem, EditProblem::Saved);
    blank_but(&notice, "2026-10-22 14:05 UTC");
}

#[test]
fn the_saved_notice_without_a_backup_has_no_value() {
    let notice = saved_notice(None);
    assert_eq!(notice.problem, EditProblem::Saved);
    blank_but(&notice, "");
}

#[test]
fn the_failure_notices_carry_only_the_problem() {
    let failed = save_failed_notice();
    assert_eq!(failed.problem, EditProblem::SaveFailed);
    blank_but(&failed, "");
    let reload = reload_failed_notice();
    assert_eq!(reload.problem, EditProblem::ReloadFailed);
    blank_but(&reload, "");
}
