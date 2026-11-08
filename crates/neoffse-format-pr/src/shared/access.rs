//! The reads and the raw edits of a slot, the same for every game: the value
//! of a field, its meta and lock, the row keys of a collection, and the raw
//! interface of `docs/architecture.md#edit-interfaces`. A game supplies its
//! character vocabulary through [`CharacterPart`].

use neoffse_core::{CapPolicy, Edit, EditError, FieldId, FieldMeta, Value};
use neoffse_data::GameTables;

use super::bind::{Binding, CharacterPart};
use super::items::Lists;
use super::paths::{Collection, List};
use super::read::{character_part, format_error, read_int, read_text, row_keys};
use super::rules::is_placeholder;
use super::write::{Target, set_value};
use crate::document::Document;

/// The value of `field`, over the parts of a model so that the shared summary
/// can call it with the document it is walking.
///
/// A computed part reads through [`CharacterPart::computed`] with the path of
/// its character; any other field reads the integer or the text at its path.
pub(crate) fn get_in<P: CharacterPart>(document: &mut Document, tables: &GameTables, field: &FieldId) -> Result<Value, EditError> {
    let binding = Binding::<P>::resolve(field)?;
    if let Binding::Character { id, part } = binding {
        let base = character_part(document, id, field, |_, base| Ok(base))?;
        if let Some(read) = part.computed(document, tables, id, field, base) {
            return read;
        }
    }
    let path = binding.locate(document, field)?;
    let read = if binding.is_text() {
        read_text(document, &path).map(Value::Text)
    } else {
        read_int(document, &path).map(Value::Int)
    };
    read.map_err(|source| format_error(field, source))
}

/// The meta of `field`, `None` for a path the game declares no field for (the
/// answer of the public `field_meta`). It reads no document.
pub(crate) fn field_meta<P: CharacterPart>(tables: &GameTables, field: &FieldId) -> Option<FieldMeta> {
    let binding = Binding::<P>::resolve(field).ok()?;
    Some(binding.meta(tables))
}

/// True when the game interface refuses every `Set` of `field`: a locked
/// character part, the count of a key item or of a placeholder row. It reads
/// the tables, not the document. `UnknownField` for an undeclared path.
pub(crate) fn field_locked<P: CharacterPart>(tables: &GameTables, field: &FieldId) -> Result<bool, EditError> {
    Ok(match Binding::<P>::resolve(field)? {
        Binding::Character { part, .. } => part.is_locked(),
        Binding::Count { list, key } => list == List::KeyItems || is_placeholder(tables, list, key),
        Binding::Gil => false,
    })
}

/// The row keys of `collection` in document order. `NotACollection` when it
/// is none of the three shared collections, `Format` when a read fails.
pub(crate) fn rows(document: &mut Document, collection: &FieldId) -> Result<Vec<u32>, EditError> {
    let kind = Collection::of(collection).ok_or_else(|| EditError::NotACollection { field: collection.clone() })?;
    row_keys(document, kind).map_err(|source| format_error(collection, source))
}

/// Applies `edit` through the raw interface and returns the edit that undoes
/// it. `own` sees the edit first: a game answers the collection it adds with
/// `Some`, and `None` passes the edit to the shared code.
pub(crate) fn apply_raw<P, O>(document: &mut Document, tables: &'static GameTables, edit: &Edit, policy: CapPolicy, own: O) -> Result<Edit, EditError>
where
    P: CharacterPart,
    O: FnOnce(&mut Document, &Edit) -> Option<Result<Edit, EditError>>,
{
    if let Some(result) = own(document, edit) {
        return result;
    }
    match edit {
        Edit::Set { field, value } => raw_set::<P>(document, tables, field, value, policy),
        Edit::Insert { row, position, values } => Lists::new(document, tables).insert(row, *position, values, policy),
        Edit::Remove { row } => Lists::new(document, tables).remove(row),
        _ => Err(EditError::ReadOnly { field: edit.target().clone() }),
    }
}

fn raw_set<P: CharacterPart>(document: &mut Document, tables: &GameTables, field: &FieldId, value: &Value, policy: CapPolicy) -> Result<Edit, EditError> {
    let binding = Binding::<P>::resolve(field)?;
    let meta = binding.meta(tables);
    // A read-only field refuses before any document read, so a slot of
    // another shape still answers `ReadOnly`.
    if meta.read_only {
        return Err(EditError::ReadOnly { field: field.clone() });
    }
    let path = binding.locate(document, field)?;
    let target = Target {
        field,
        meta: &meta,
        path: &path,
        text: binding.is_text(),
    };
    set_value(document, tables, &target, value, policy)
}
