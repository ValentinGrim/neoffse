//! The stored places of a spell, one field each, for the expert pages.
//!
//! A spell lives in several places: an entry of `abilityList`, a copy in
//! `abilityDictionary` key 3, and a content id in each of the two order
//! lists (`docs/formats/pixel-remaster.md#row-and-value-in-ff-ii`). Each
//! place is a field addressed by its position in its list, counted from 0.
//! An ability entry has no key but the value these fields edit, so a path
//! by id would lose the entry the moment its id changes
//! (`docs/formats/pixel-remaster.md#abilities`): the inverse of every raw
//! `Set` is the `Set` of the old value at the same position.

use neoffse_core::{EditError, FieldId, FieldMeta, FieldView, Key, TableKind, Value};
use neoffse_data::{GameTables, Language};

use super::meta::level_total;
use super::names;
use super::spell::ability_label;
use crate::document::{Document, DocumentError, Segment};
use crate::shared::meta::int;
use crate::shared::paths::{Path, element, with};
use crate::shared::read::{Located, character_part, entry_value, format_error, read_int, read_stored};

/// The key of the Magic command in `abilityDictionary`: its list holds the
/// spells (`docs/formats/pixel-remaster.md#abilities`).
pub(super) const MAGIC_COMMAND: u32 = 3;

/// A list of ability entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Group {
    /// `abilityList`.
    Ability,
    /// The `abilityDictionary` list under `MAGIC_COMMAND`.
    Dictionary,
}

/// The three values of an ability entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntryPart {
    /// `abilityId`.
    Ability,
    /// `contentId`.
    Content,
    /// `skillLevel`, the proficiency total.
    Proficiency,
}

impl EntryPart {
    const ALL: [Self; 3] = [Self::Ability, Self::Content, Self::Proficiency];

    fn of(name: &str) -> Option<Self> {
        match name {
            names::ABILITY_ID => Some(Self::Ability),
            names::CONTENT_ID => Some(Self::Content),
            names::PROFICIENCY => Some(Self::Proficiency),
            _ => None,
        }
    }

    /// The field name after the position.
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Ability => names::ABILITY_ID,
            Self::Content => names::CONTENT_ID,
            Self::Proficiency => names::PROFICIENCY,
        }
    }

    /// The key of the entry's JSON object.
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Ability => "abilityId",
            Self::Content => "contentId",
            Self::Proficiency => "skillLevel",
        }
    }

    /// The last word of a view label.
    fn word(self) -> &'static str {
        match self {
            Self::Ability => "row",
            Self::Content => "content",
            Self::Proficiency => "value",
        }
    }
}

/// A list of content ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Order {
    /// `additionOrderOwnedAbilityIds`.
    Addition,
    /// `sortOrderOwnedAbilityIds`.
    Sort,
}

impl Order {
    /// The path to the array of content ids for the character at `base`.
    fn list(self, base: &Path) -> Path {
        let key = match self {
            Self::Addition => "additionOrderOwnedAbilityIds",
            Self::Sort => "sortOrderOwnedAbilityIds",
        };
        with(base.clone(), &[Segment::Key(key), Segment::Doc, Segment::Key("target")])
    }

    fn name(self) -> &'static str {
        match self {
            Self::Addition => names::ADDITION_ORDER,
            Self::Sort => names::SORT_ORDER,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Addition => "Addition order",
            Self::Sort => "Sort order",
        }
    }
}

/// One stored place of a spell, by position in its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Place {
    Entry { group: Group, position: u32, part: EntryPart },
    Order { order: Order, position: u32 },
}

impl Place {
    /// The place that `keys` (after `character.<id>`) name, `None` for any other path.
    pub(super) fn declared(keys: &[Key]) -> Option<Self> {
        let entry = |group, position: &u32, part: &&str| {
            Some(Self::Entry {
                group,
                position: *position,
                part: EntryPart::of(part)?,
            })
        };
        match keys {
            [Key::Name(names::ABILITY), Key::Id(position), Key::Name(part)] => entry(Group::Ability, position, part),
            [Key::Name(names::DICTIONARY), Key::Id(position), Key::Name(part)] => entry(Group::Dictionary, position, part),
            [Key::Name(names::ADDITION_ORDER), Key::Id(position)] => Some(Self::Order {
                order: Order::Addition,
                position: *position,
            }),
            [Key::Name(names::SORT_ORDER), Key::Id(position)] => Some(Self::Order {
                order: Order::Sort,
                position: *position,
            }),
            _ => None,
        }
    }
}

/// The meta of a place: an integer from 0 to the storage bound. The row
/// takes the `Ability` table, the proficiency the cap of the top spell
/// level (`docs/formats/pixel-remaster.md#skill-levels-in-ff-ii`).
pub(super) fn meta(place: Place, tables: &GameTables) -> FieldMeta {
    match place {
        Place::Entry { part, .. } => match part {
            EntryPart::Ability => {
                let mut meta = int("Ability row", 0, None);
                meta.tables = vec![TableKind::Ability];
                meta
            }
            EntryPart::Content => int("Ability content", 0, None),
            EntryPart::Proficiency => {
                let mut meta = int("Proficiency", 0, None);
                meta.cap = level_total(tables, "MAX_MAGICAL_LEVEL");
                meta
            }
        },
        Place::Order { order, .. } => int(order.label(), 0, None),
    }
}

/// The path to the array of the entries of `group` for the character at `base`.
/// `Absent` when the dictionary holds no Magic command.
fn entries(document: &mut Document, base: &Path, group: Group) -> Result<Path, Located> {
    match group {
        Group::Ability => Ok(with(base.clone(), &[Segment::Key("abilityList"), Segment::Doc, Segment::Key("target")])),
        Group::Dictionary => {
            let dictionary = with(base.clone(), &[Segment::Key("abilityDictionary"), Segment::Doc]);
            let value = entry_value(document, dictionary, MAGIC_COMMAND)?;
            Ok(with(value, &[Segment::Doc, Segment::Key("target")]))
        }
    }
}

/// The path to the value of `place` for the character at `base`. `Absent`
/// for a position past its list or a dictionary without the Magic command.
pub(super) fn path(document: &mut Document, base: &Path, place: Place) -> Result<Path, Located> {
    let (list, position) = match place {
        Place::Entry { group, position, .. } => (entries(document, base, group)?, position),
        Place::Order { order, position } => (order.list(base), position),
    };
    // A position too large for `usize` is past every list.
    let index = usize::try_from(position).map_err(|_| Located::Absent)?;
    if index >= document.array_len(&list)? {
        return Err(Located::Absent);
    }
    Ok(match place {
        Place::Entry { part, .. } => with(element(list, index), &[Segment::Key(part.key())]),
        Place::Order { .. } => with(list, &[Segment::Index(index)]),
    })
}

/// A list's path and length. A list the character lacks is empty: the
/// character owns no such place (`docs/formats/pixel-remaster.md#abilities`).
/// `Format` for any other read failure.
fn counted(document: &mut Document, list: Result<Path, Located>, field: &FieldId) -> Result<Option<(Path, usize)>, EditError> {
    let list = match list {
        Ok(list) => list,
        Err(Located::Absent | Located::Failed(DocumentError::MissingKey { .. })) => return Ok(None),
        Err(Located::Failed(source)) => return Err(format_error(field, source)),
    };
    match document.array_len(&list) {
        Ok(len) => Ok(Some((list, len))),
        Err(DocumentError::MissingKey { .. }) => Ok(None),
        Err(source) => Err(format_error(field, source)),
    }
}

/// The path and the length of the entries of `group` for the character at
/// `base`, `None` when the character has no such list.
pub(super) fn entries_of(document: &mut Document, base: &Path, group: Group, field: &FieldId) -> Result<Option<(Path, usize)>, EditError> {
    let list = entries(document, base, group);
    counted(document, list, field)
}

/// The view label of an entry: its ability's name, `Entry <id>` without one.
fn entry_label(tables: &GameTables, language: Language, ability: i64) -> String {
    match u32::try_from(ability) {
        Ok(id) => ability_label(tables, language, id),
        // An id outside `u32` is in no table.
        Err(_) => format!("Entry {ability}"),
    }
}

/// The place fields of character `id`: the `abilityList` entries, the
/// dictionary entries, the addition order, then the sort order. A place the
/// save lacks is left out. The labels follow the document order, so the
/// three fields of an entry sit together:
/// `List 1, Entry 711, row`, `Copy 2, Fire, value`, `Sort order 3`.
pub(super) fn fields(document: &mut Document, tables: &GameTables, id: u32, language: Language) -> Result<Vec<FieldView>, EditError> {
    let character = FieldId::new(names::CHARACTER).with_id(id);
    let base = character_part(document, id, &character, |_, base| Ok(base))?;
    let mut fields = Vec::new();
    for (group, prefix) in [(Group::Ability, "List"), (Group::Dictionary, "Copy")] {
        let Some((list, len)) = entries_of(document, &base, group, &character)? else {
            continue;
        };
        let group_name = if group == Group::Ability { names::ABILITY } else { names::DICTIONARY };
        for (index, position) in (0..len).zip(0_u32..) {
            let entry = element(list.clone(), index);
            let ability =
                read_int(document, &with(entry.clone(), &[Segment::Key(EntryPart::Ability.key())])).map_err(|source| format_error(&character, source))?;
            let name = entry_label(tables, language, ability);
            for part in EntryPart::ALL {
                let stored = if part == EntryPart::Ability {
                    Some(ability)
                } else {
                    read_stored(document, &with(entry.clone(), &[Segment::Key(part.key())]), &character)?
                };
                let Some(value) = stored else { continue };
                let mut meta = meta(Place::Entry { group, position, part }, tables);
                meta.label = format!("{prefix} {}, {name}, {}", position + 1, part.word());
                fields.push(FieldView::new(names::place(id, group_name, position, part.name()), meta, Value::Int(value)));
            }
        }
    }
    for order in [Order::Addition, Order::Sort] {
        let Some((list, len)) = counted(document, Ok(order.list(&base)), &character)? else {
            continue;
        };
        for (index, position) in (0..len).zip(0_u32..) {
            let value = read_int(document, &with(list.clone(), &[Segment::Index(index)])).map_err(|source| format_error(&character, source))?;
            let mut meta = meta(Place::Order { order, position }, tables);
            meta.label = format!("{} {}", order.label(), position + 1);
            fields.push(FieldView::new(names::order_place(id, order.name(), position), meta, Value::Int(value)));
        }
    }
    Ok(fields)
}
