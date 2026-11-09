//! The Level edit of a spell: the plain `Set`s that one level change writes.
//!
//! The game changes five things for one spell of one character
//! (`docs/formats/pixel-remaster.md#row-and-value-in-ff-ii`): the chain row
//! and its content id, `skillLevel` set to the proficiency total of the
//! level, the same three values on the `abilityDictionary` copy, and the old
//! content id in the two order lists, replaced in place when the list holds
//! it. The plan finds each position once, before any write, and emits one
//! `Set` per place. Every place is addressed by its position in its list, so
//! the inverse of each `Set` is the `Set` of the old value at that position.
//! The game appends the new content id to an order list that lacks the old
//! one; NeoFFSE leaves such a list as it is.

use neoffse_core::{Edit, EditError, FieldId, TableKind, Value};
use neoffse_data::GameTables;

use super::Ff2Model;
use super::places::{EntryPart, Group, Order, Place, order_of, path as place_path};
use super::spell::spell_of;
use crate::document::{Document, Segment};
use crate::shared::paths::{Path, with};
use crate::shared::plan::Plan;
use crate::shared::read::{character_part, format_error, read_int};

/// The ability row, its content id and the proficiency total of one level.
struct Row {
    ability: i64,
    content: i64,
    total: i64,
}

/// The row of `level` in `chain`. `UnknownId` when the tables hold no such
/// row, no content id for it, or no proficiency total for the level.
fn row_of(tables: &GameTables, chain: u32, level: i64, field: &FieldId) -> Result<Row, EditError> {
    let unknown = |id: i64| EditError::UnknownId { field: field.clone(), id };
    // The meta bounds `level` to the chain length before the plan runs; a level
    // outside `u32` is in no chain, like one past its end.
    let level_id = u32::try_from(level).map_err(|_| unknown(level))?;
    let ability = tables.spell_ability(chain, level_id).ok_or_else(|| unknown(level))?;
    let content = tables
        .entry(TableKind::Ability, ability)
        .and_then(|entry| entry.ability)
        .and_then(|info| info.content_id)
        .ok_or_else(|| unknown(i64::from(ability)))?;
    // The parser ties each chain's length to the skill curve, so every level of a chain has a total; a `None` stays `UnknownId` and writes nothing.
    let total = tables.skill_total(level_id).ok_or_else(|| unknown(level))?;
    Ok(Row {
        ability: i64::from(ability),
        content: i64::from(content),
        total: i64::from(total),
    })
}

/// The stored integer at `place` of the character at `base`.
fn stored(document: &mut Document, base: &Path, place: Place, field: &FieldId) -> Result<i64, EditError> {
    let value = place_path(document, base, place).map_err(|error| error.into_error(field))?;
    read_int(document, &value).map_err(|source| format_error(field, source))
}

/// The position of the first element of `order` that holds `content`.
fn order_position(document: &mut Document, base: &Path, order: Order, content: i64, field: &FieldId) -> Result<Option<u32>, EditError> {
    let Some((list, len)) = order_of(document, base, order, field)? else {
        return Ok(None);
    };
    for (index, position) in (0..len).zip(0_u32..) {
        let element = with(list.clone(), &[Segment::Index(index)]);
        if read_int(document, &element).map_err(|source| format_error(field, source))? == content {
            return Ok(Some(position));
        }
    }
    Ok(None)
}

impl Ff2Model {
    /// The plain edits that set the level of spell `chain` of character `id`
    /// to `level`, in apply order: the `abilityList` entry (row, content id,
    /// proficiency), the dictionary copy (the same three), then the addition
    /// order and the sort order when they hold the old content id. A place
    /// that already holds its target value is left out, except the first
    /// edit, so a plan is never empty. No write happens here.
    ///
    /// `UnknownField` when the character owns no entry of the chain in
    /// `abilityList`, or none in the key-3 copy; `UnknownId` when the tables
    /// hold no row, content id or total for the level.
    pub(super) fn spell_plan(&mut self, field: &FieldId, id: u32, chain: u32, level: i64) -> Result<Plan, EditError> {
        let tables = self.tables;
        let document = &mut self.document;
        let base = character_part(document, id, field, |_, base| Ok(base))?;
        let listed = spell_of(document, tables, &base, Group::Ability, chain, field)?.position;
        let copied = spell_of(document, tables, &base, Group::Dictionary, chain, field)?.position;
        let row = row_of(tables, chain, level, field)?;
        let entry = |group, position| Place::Entry {
            group,
            position,
            part: EntryPart::Content,
        };
        let old_content = stored(document, &base, entry(Group::Ability, listed), field)?;

        let mut edits: Vec<Edit> = Vec::new();
        let mut push = |document: &mut Document, place: Place, target: i64| -> Result<(), EditError> {
            if edits.is_empty() || stored(document, &base, place, field)? != target {
                edits.push(Edit::set(place.field(id), Value::Int(target)));
            }
            Ok(())
        };
        for (group, position) in [(Group::Ability, listed), (Group::Dictionary, copied)] {
            for (part, target) in [
                (EntryPart::Ability, row.ability),
                (EntryPart::Content, row.content),
                (EntryPart::Proficiency, row.total),
            ] {
                push(document, Place::Entry { group, position, part }, target)?;
            }
        }
        for order in [Order::Addition, Order::Sort] {
            if let Some(position) = order_position(document, &base, order, old_content, field)? {
                push(document, Place::Order { order, position }, row.content)?;
            }
        }
        Ok(edits.into())
    }
}
