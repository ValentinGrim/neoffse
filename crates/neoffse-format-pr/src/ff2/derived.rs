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
use crate::shared::derived::{held_to_cap, status_base, status_id};
use crate::shared::paths::with;
use crate::shared::read::{character_part, read_stored};

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
    let base = status_base(tables, status, |tables, id| tables.base_stats(id).map(|stats| base_of(&stats)))?;
    Some(held_to_cap(base, gain?, tables.bound(cap_key)?))
}
