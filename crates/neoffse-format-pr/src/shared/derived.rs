//! The reads and the clamp behind a computed figure of a character
//! (`max_hp`, `max_mp`): the status id, its base row and the capped sum.
//! The formulas stay in the game modules.

use neoffse_core::{EditError, FieldId};
use neoffse_data::GameTables;

use super::paths::{Path, with};
use super::read::read_stored;
use crate::document::{Document, Segment};

/// The `characterStatusId` of the character at `base`, `None` when the key is
/// missing. Any other failure is the `Format` error of `field`.
pub(crate) fn status_id(document: &mut Document, base: &Path, field: &FieldId) -> Result<Option<i64>, EditError> {
    read_stored(document, &with(base.clone(), &[Segment::Key("characterStatusId")]), field)
}

/// The base figure of the status row, read by `base_of` from the tables.
/// `None` for a missing status, a negative one, one above `u32` and one with
/// no row: the caller reports each as `UnknownField` (the row is absent).
pub(crate) fn status_base(tables: &GameTables, status: Option<i64>, base_of: impl FnOnce(&GameTables, u32) -> Option<u32>) -> Option<u32> {
    base_of(tables, u32::try_from(status?).ok()?)
}

/// The base plus the stored gain, saturated and held to `cap`: the cap limits
/// the sum the menu shows, not the stored gain
/// (`docs/formats/pixel-remaster.md#bounds-in-the-master-data`). The lower
/// bound only guards a hand-edited negative gain, and a negative cap counts
/// as 0.
pub(crate) fn held_to_cap(base: u32, gain: i64, cap: i64) -> i64 {
    i64::from(base).saturating_add(gain).clamp(0, cap.max(0))
}
