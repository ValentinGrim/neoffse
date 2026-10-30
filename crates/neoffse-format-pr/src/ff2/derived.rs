//! The two computed fields of a character, `max_hp` and `max_mp`.
//!
//! Both come from the document and the tables and write nothing. The rule is
//! the stored-gain reading of
//! `docs/formats/pixel-remaster.md#character-growth-in-ff-ii`
//! ("Reading a stored gain"): the game shows `min(max(base + gain, 0), cap)`.

use neoffse_core::{EditError, FieldId, Value};
use neoffse_data::{BaseStats, GameTables};

use super::binding::Part;
use crate::document::{Document, Segment};
use crate::shared::paths::{Path, with};
use crate::shared::read::{character_part, read_stored};

/// The `characterStatusId` of the character at `base`, `None` when the key is
/// missing. Any other failure is the `Format` error of `field`.
pub(super) fn status_id(document: &mut Document, base: &Path, field: &FieldId) -> Result<Option<i64>, EditError> {
    read_stored(document, &with(base.clone(), &[Segment::Key("characterStatusId")]), field)
}

/// The value of the computed `part` of character `id`.
///
/// `UnknownField` when the character lacks a key the figure reads, or when a
/// table finds no row: a status id outside `character_status`, no cap key.
/// `Format` when a key holds a value of another kind.
pub(super) fn value(document: &mut Document, tables: &GameTables, id: u32, part: Part, field: &FieldId) -> Result<Value, EditError> {
    let base = character_part(document, id, field, |_, base| Ok(base))?;
    let (gain_key, cap_key, base_of): (_, _, fn(&BaseStats) -> u32) = match part {
        Part::MaxHp => ("addtionalMaxHp", "MAX_HP", |stats| stats.hp),
        Part::MaxMp => ("addtionalMaxMp", "MAX_MP", |stats| stats.mp),
        _ => return Err(EditError::UnknownField { field: field.clone() }),
    };
    let status = status_id(document, &base, field)?;
    let parameter = with(base, &[Segment::Key("parameter"), Segment::Doc]);
    let gain = read_stored(document, &with(parameter, &[Segment::Key(gain_key)]), field)?;
    maximum(tables, status, gain, cap_key, base_of)
        .map(Value::Int)
        .ok_or_else(|| EditError::UnknownField { field: field.clone() })
}

/// The base of the status row plus the stored gain, saturated and held to
/// the cap: the cap limits the sum the menu shows, not the stored gain. The
/// lower bound only guards a hand-edited negative gain.
fn maximum(tables: &GameTables, status: Option<i64>, gain: Option<i64>, cap_key: &str, base_of: fn(&BaseStats) -> u32) -> Option<i64> {
    // A status id that is negative, above `u32` or without a row gives `None`,
    // which the caller reports as `UnknownField` (the row is absent).
    let base = base_of(&tables.base_stats(u32::try_from(status?).ok()?)?);
    let cap = tables.bound(cap_key)?.max(0);
    Some(i64::from(base).saturating_add(gain?).clamp(0, cap))
}
