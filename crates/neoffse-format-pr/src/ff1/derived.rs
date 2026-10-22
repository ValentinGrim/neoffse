//! The two computed fields of a character, `max_hp` and `next_level`.
//!
//! Both come from the document and the tables and write nothing. The rules
//! are in `docs/formats/pixel-remaster.md#maximum-hp-and-experience-tables`
//! (tables, "Next-level figure") and `docs/formats/pixel-remaster.md#characters`
//! (the `parameter` formulas).

use neoffse_core::{EditError, FieldId, Value};
use neoffse_data::GameTables;

use super::binding::{Collection, Part, characters_target, element, with};
use super::locate::{find_row, format_error, missing_row, read_int};
use super::names;
use crate::document::{Document, DocumentError, Segment};

/// The value of the computed `part` of character `id`.
///
/// `UnknownField` when the character lacks a key the figure reads, or when a
/// table finds no row: a status id outside `character_status`, no `MAX_HP`
/// cap, an experience with no next level row. `Format` when a key holds a
/// value of another kind.
pub(super) fn value(document: &mut Document, tables: &GameTables, id: u32, part: Part, field: &FieldId) -> Result<Value, EditError> {
    let failed = |source| format_error(field, source);
    let index = find_row(document, Collection::Character, id)
        .map_err(failed)?
        .ok_or_else(|| missing_row(names::CHARACTER, id))?;
    let base = element(characters_target(), index);
    let absent = || EditError::UnknownField { field: field.clone() };
    let figure = match part {
        Part::MaxHp => {
            let status = read_stored(document, &with(base.clone(), &[Segment::Key("characterStatusId")]), field)?;
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

/// An integer of the character, `None` when its key is missing.
fn read_stored(document: &mut Document, path: &[Segment<'_>], field: &FieldId) -> Result<Option<i64>, EditError> {
    match read_int(document, path) {
        Ok(number) => Ok(Some(number)),
        Err(DocumentError::MissingKey { .. }) => Ok(None),
        Err(source) => Err(format_error(field, source)),
    }
}

/// The base HP of the status row plus the stored gain, saturated and held to
/// `MAX_HP`: the cap limits the sum the menu shows, not the stored gain
/// (`docs/formats/pixel-remaster.md#bounds-in-the-master-data`). The lower
/// bound only guards a hand-edited negative gain.
fn max_hp(tables: &GameTables, status: Option<i64>, gain: Option<i64>) -> Option<i64> {
    let base = tables.base_hp(u32::try_from(status?).ok()?)?;
    let cap = tables.bound("MAX_HP")?.max(0);
    Some(i64::from(base).saturating_add(gain?).clamp(0, cap))
}

/// The experience still needed for the next level. The level comes from the
/// experience alone, never from the stored `addtionalLevel`: the first total
/// above `exp` is the one of level L + 1, and level 1 holds 0. `None` for a
/// negative experience and for L = 99, which has no next row.
fn next_level(tables: &GameTables, exp: Option<i64>) -> Option<i64> {
    let exp = u32::try_from(exp?).ok()?;
    let next = (2..).map_while(|level| tables.exp_to_reach(level)).find(|total| *total > exp)?;
    Some(i64::from(next - exp))
}
