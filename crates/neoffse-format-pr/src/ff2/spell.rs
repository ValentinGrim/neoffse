//! The spells of a character: the Level field and the Magic section.
//!
//! A spell is one of 40 chains of 16 ability rows, and its level is the
//! position of its row in the chain
//! (`docs/formats/pixel-remaster.md#spells-in-ff-ii`). NeoFFSE reads the level
//! from the row of the `abilityList` entry, never from `skillLevel`: the two
//! disagree on a pair an editor wrote
//! (`docs/formats/pixel-remaster.md#row-and-value-in-ff-ii`). A read changes
//! no byte.

use neoffse_core::{EditError, FieldId, FieldView, TableKind, Value};
use neoffse_data::{GameTables, Language};

use super::binding::{Binding, Part};
use super::meta;
use super::places::{Group, entries_of};
use crate::document::{Document, Segment};
use crate::shared::paths::{Path, element, with};
use crate::shared::read::{character_part, format_error, read_int};

/// One spell a character owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Owned {
    /// The position of the entry in its list, counted from 0.
    pub position: u32,
    /// The chain, 0 to 39.
    pub chain: u32,
    /// The level of the row the `abilityList` entry holds, 1 to 16.
    pub level: u32,
    /// The first ability id of the chain: the row that carries the name.
    pub first: u32,
}

/// The spells in the list `group` of the character at `base`, in list
/// order, one per chain: the first entry of a chain wins. An entry outside
/// every chain (ability 711, rows 645 to 805) is not a spell and is skipped.
///
/// `Format` for an entry without an `abilityId` or with a non-integer one.
fn owned(document: &mut Document, tables: &GameTables, base: &Path, group: Group, field: &FieldId) -> Result<Vec<Owned>, EditError> {
    // A character without the list owns no spell: the field is absent
    // (`docs/formats/pixel-remaster.md#abilities`).
    let Some((target, len)) = entries_of(document, base, group, field)? else {
        return Ok(Vec::new());
    };
    let mut spells: Vec<Owned> = Vec::new();
    for (index, position) in (0..len).zip(0_u32..) {
        let path = with(element(target.clone(), index), &[Segment::Key("abilityId")]);
        let ability = read_int(document, &path).map_err(|source| format_error(field, source))?;
        // An id outside `u32` is in no chain, like 711.
        let Some((id, step)) = u32::try_from(ability).ok().and_then(|id| Some((id, tables.spell_step(id)?))) else {
            continue;
        };
        if spells.iter().all(|spell| spell.chain != step.chain) {
            // `spell_step` defines `level = id - first + 1`, so the first id is `id - (level - 1)`.
            let first = id.saturating_sub(step.level.saturating_sub(1));
            spells.push(Owned {
                position,
                chain: step.chain,
                level: step.level,
                first,
            });
        }
    }
    Ok(spells)
}

/// The first entry of `chain` in the list `group` of the character at
/// `base`. `UnknownField` when the list holds none, or the character has no
/// such list.
pub(super) fn spell_of(document: &mut Document, tables: &GameTables, base: &Path, group: Group, chain: u32, field: &FieldId) -> Result<Owned, EditError> {
    owned(document, tables, base, group, field)?
        .into_iter()
        .find(|spell| spell.chain == chain)
        .ok_or_else(|| EditError::UnknownField { field: field.clone() })
}

/// The level of spell `chain` of character `id`, as a `Value`.
/// `UnknownField` when the character owns no entry of that chain.
pub(super) fn level(document: &mut Document, tables: &GameTables, id: u32, chain: u32, field: &FieldId) -> Result<Value, EditError> {
    let base = character_part(document, id, field, |_, base| Ok(base))?;
    let spell = spell_of(document, tables, &base, Group::Ability, chain, field)?;
    Ok(Value::Int(i64::from(spell.level)))
}

/// The name of ability `id` in `language`, `Entry <id>` when its row has no
/// name (`docs/formats/pixel-remaster.md#rows-without-a-name`).
pub(super) fn ability_label(tables: &GameTables, language: Language, id: u32) -> String {
    tables
        .entry(TableKind::Ability, id)
        .and_then(|entry| entry.name(language))
        .filter(|name| !name.is_empty())
        .map_or_else(|| format!("Entry {id}"), str::to_owned)
}

/// The Level fields of character `id`, one per owned spell in `abilityList`
/// order. Each label is the spell's name: the 16 rows of a chain share one
/// name, so the row of level 1 gives it.
pub(super) fn fields(document: &mut Document, tables: &GameTables, id: u32, language: Language) -> Result<Vec<FieldView>, EditError> {
    let character = FieldId::new(super::names::CHARACTER).with_id(id);
    let base = character_part(document, id, &character, |_, base| Ok(base))?;
    let spells = owned(document, tables, &base, Group::Ability, &character)?;
    Ok(spells
        .into_iter()
        .map(|spell| {
            let mut meta = meta::field_meta(
                &Binding::Character {
                    id,
                    part: Part::Spell(spell.chain),
                },
                tables,
            );
            meta.label = ability_label(tables, language, spell.first);
            FieldView::new(super::names::spell_field(id, spell.chain), meta, Value::Int(i64::from(spell.level)))
        })
        .collect())
}
