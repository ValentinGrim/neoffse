//! The pure functions behind the review dialog: the rows of the change list
//! and the time of a backup.
//!
//! Both take plain data and return plain data, so a test calls them without a
//! session or a window. Nothing here names a game or a field
//! (`docs/architecture.md#ui-architecture`); `overview.rs` stays the one
//! module that does.

use neoffse::core::{Capability, FieldId};
use neoffse::io::backup::Moment;
use neoffse::{Change, ChangeKind};

use crate::{ReviewKind, ReviewRow};

/// The rows of the review list: one heading row per run of changes of the same
/// capability and entity, then one row per change.
///
/// A run ends when the pair (capability, entity id) differs from the previous
/// change's pair, so the same entity in two separate runs gets two headings.
/// The heading shows the entity's label. A `Field` change gives a `field` row
/// with entity, label, before and after; a row added or removed gives an
/// `added` or `removed` row with the entity set. Unused strings are empty.
///
/// The rows hold save content: never log them.
#[must_use]
pub fn review_rows(changes: &[Change]) -> Vec<ReviewRow> {
    let mut rows = Vec::with_capacity(changes.len().saturating_mul(2));
    let mut previous: Option<(Capability, &FieldId)> = None;
    for change in changes {
        let pair = (change.capability, &change.entity);
        if previous != Some(pair) {
            rows.push(row(ReviewKind::Heading, &change.entity_label));
            previous = Some(pair);
        }
        rows.push(match &change.kind {
            ChangeKind::Field { label, before, after, .. } => ReviewRow {
                label: label.as_str().into(),
                before: before.as_str().into(),
                after: after.as_str().into(),
                ..row(ReviewKind::Field, &change.entity_label)
            },
            ChangeKind::RowRemoved => row(ReviewKind::Removed, &change.entity_label),
            // `RowAdded` and any kind a later version adds.
            _ => row(ReviewKind::Added, &change.entity_label),
        });
    }
    rows
}

fn row(kind: ReviewKind, entity: &str) -> ReviewRow {
    ReviewRow {
        kind,
        entity: entity.into(),
        ..ReviewRow::default()
    }
}

/// "2026-10-22 14:05 UTC" from the `created` text of a backup manifest: the
/// date, the time to the minute and the zone ("UTC" for `Z`, else the offset
/// as written).
///
/// `Moment::parse` validates the text first and `None` answers its refusal.
/// The result cuts the validated text and does no calendar arithmetic, so the
/// time stays in the zone the manifest recorded.
#[must_use]
pub fn stamp_of(created: &str) -> Option<String> {
    Moment::parse(created)?;
    let date = created.get(..10)?;
    let time = created.get(11..16)?;
    let zone = match created.get(19..)? {
        "Z" => "UTC",
        offset => offset,
    };
    Some(format!("{date} {time} {zone}"))
}
