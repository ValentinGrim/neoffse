//! The two computed fields of a character, `max_hp` and `next_level`.
//!
//! Both come from the document and the tables and write nothing. The rules
//! are in `docs/formats/pixel-remaster.md#maximum-hp-and-experience-tables`
//! (tables, "Next-level figure") and `docs/formats/pixel-remaster.md#characters`
//! (the `parameter` formulas).

use neoffse_core::{EditError, FieldId, Value};
use neoffse_data::GameTables;

use super::binding::Part;
use crate::document::{Document, Segment};
use crate::shared::derived::{held_to_cap, status_base, status_id};
use crate::shared::paths::with;
use crate::shared::read::{character_part, read_stored};

/// The value of the computed `part` of character `id`.
///
/// `UnknownField` when the character lacks a key the figure reads, or when a
/// table finds no row: a status id outside `character_status`, no `MAX_HP`
/// cap, an experience with no next level row. `Format` when a key holds a
/// value of another kind.
pub(super) fn value(document: &mut Document, tables: &GameTables, id: u32, part: Part, field: &FieldId) -> Result<Value, EditError> {
    let base = character_part(document, id, field, |_, base| Ok(base))?;
    let absent = || EditError::UnknownField { field: field.clone() };
    let figure = match part {
        Part::MaxHp => {
            let status = status_id(document, &base, field)?;
            let parameter = with(base, &[Segment::Key("parameter"), Segment::Doc]);
            let gain = read_stored(document, &with(parameter, &[Segment::Key("addtionalMaxHp")]), field)?;
            max_hp(tables, status, gain)
        }
        Part::NextLevel => {
            let exp = read_stored(document, &with(base, &[Segment::Key("currentExp")]), field)?;
            next_level(tables, exp)
        }
        _ => None,
    };
    figure.map(Value::Int).ok_or_else(absent)
}

/// The base HP of the status row plus the stored gain, saturated and held to
/// `MAX_HP`: the cap limits the sum the menu shows, not the stored gain
/// (`docs/formats/pixel-remaster.md#bounds-in-the-master-data`). The lower
/// bound only guards a hand-edited negative gain.
fn max_hp(tables: &GameTables, status: Option<i64>, gain: Option<i64>) -> Option<i64> {
    let base = status_base(tables, status, GameTables::base_hp)?;
    Some(held_to_cap(base, gain?, tables.bound("MAX_HP")?))
}

/// The highest level whose total experience is at most `exp`, found in the
/// ascending totals of the experience table
/// (`docs/formats/pixel-remaster.md#maximum-hp-and-experience-tables`). Level 1
/// holds 0, so the answer is at least 1 for any table with rows; `None` only
/// for an empty table.
pub(super) fn level_for(tables: &GameTables, exp: u32) -> Option<u32> {
    (1..)
        .map_while(|level| tables.exp_to_reach(level).map(|total| (level, total)))
        .take_while(|(_, total)| *total <= exp)
        .last()
        .map(|(level, _)| level)
}

/// The experience still needed for the next level. The level comes from the
/// experience alone, never from the stored `addtionalLevel`: the total of
/// level L + 1 minus `exp`. `None` for a negative experience and for L = 99,
/// which has no next row.
fn next_level(tables: &GameTables, exp: Option<i64>) -> Option<i64> {
    let exp = u32::try_from(exp?).ok()?;
    let next = tables.exp_to_reach(level_for(tables, exp)?.checked_add(1)?)?;
    Some(i64::from(next - exp))
}
