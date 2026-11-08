//! From a [`FieldId`] to what it names: gil, a part of a character, or the
//! count of an item row. A game supplies the character vocabulary through
//! [`CharacterPart`]; the rest of the binding is the same for every game.

use neoffse_core::{EditError, FieldId, FieldMeta, Key, Value};
use neoffse_data::GameTables;

use super::meta::{count_meta, gil_meta};
use super::names;
use super::paths::{List, Path, gil_path};
use super::read::{Located, character_part, count_path};
use crate::document::Document;

/// The field vocabulary of one game's characters.
pub(crate) trait CharacterPart: Copy {
    /// The part named by the keys that follow `character.<id>`, `None` for
    /// keys the game declares no field for.
    fn declared(keys: &[Key]) -> Option<Self>;

    /// The meta of the part. It reads no document.
    fn meta(self, tables: &GameTables) -> FieldMeta;

    /// The path to the part, from the element `base` of the character.
    /// `Located::Absent` for a dictionary without the key.
    fn locate(self, document: &mut Document, base: Path) -> Result<Path, Located>;

    /// True for the one text field, `name`.
    fn is_text(self) -> bool;

    /// True for a part with no document path of its own: [`Binding::locate`]
    /// answers `ReadOnly` for it before any read.
    fn is_pathless(self) -> bool;

    /// True when `apply_game` refuses every `Set` of the part, whatever the
    /// value.
    fn is_locked(self) -> bool;

    /// A read that bypasses [`locate`](Self::locate): a computed field, or a
    /// value that sits inside a JSON string. `None` for the generic read.
    fn computed(self, document: &mut Document, tables: &GameTables, id: u32, field: &FieldId) -> Option<Result<Value, EditError>>;
}

/// What a [`FieldId`] names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Binding<P> {
    Gil,
    Character { id: u32, part: P },
    Count { list: List, key: u32 },
}

impl<P: CharacterPart> Binding<P> {
    /// Resolves `field`, or `UnknownField` when it matches no declared field.
    pub(crate) fn resolve(field: &FieldId) -> Result<Self, EditError> {
        Self::declared(field.keys()).ok_or_else(|| EditError::UnknownField { field: field.clone() })
    }

    fn declared(keys: &[Key]) -> Option<Self> {
        match keys {
            [Key::Name(names::GIL)] => Some(Self::Gil),
            [Key::Name(names::CHARACTER), Key::Id(id), rest @ ..] => Some(Self::Character {
                id: *id,
                part: P::declared(rest)?,
            }),
            _ => List::of_count_field(keys).map(|(list, key)| Self::Count { list, key }),
        }
    }

    /// The meta of the field that the binding names.
    pub(crate) fn meta(&self, tables: &GameTables) -> FieldMeta {
        match *self {
            Self::Gil => gil_meta(tables),
            Self::Count { list, .. } => count_meta(list, tables),
            Self::Character { part, .. } => part.meta(tables),
        }
    }

    /// The path to the value of the binding. `MissingRow` for an absent row,
    /// `UnknownField` for a dictionary without the key, `Format` for a read
    /// that fails, `ReadOnly` for a pathless part.
    pub(crate) fn locate(&self, document: &mut Document, field: &FieldId) -> Result<Path, EditError> {
        match *self {
            Self::Gil => Ok(gil_path()),
            Self::Count { list, key } => count_path(document, list, key, field),
            Self::Character { id, part } => {
                if part.is_pathless() {
                    return Err(EditError::ReadOnly { field: field.clone() });
                }
                character_part(document, id, field, |document, base| part.locate(document, base))
            }
        }
    }

    /// True for the one text field, `name`.
    pub(crate) fn is_text(&self) -> bool {
        matches!(self, Self::Character { part, .. } if part.is_text())
    }
}
