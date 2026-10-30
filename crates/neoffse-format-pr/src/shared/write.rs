//! The raw `Set`: the order of checks and the write once a field is located.

use neoffse_core::{CapPolicy, Edit, EditError, FieldId, FieldMeta, Value};
use neoffse_data::GameTables;

use super::read::{format_error, read_int, read_text};
use crate::document::{Document, Scalar, Segment};

/// A located field: its id, its meta, its document path, and whether the
/// stored value is text (else an integer).
pub(crate) struct Target<'a> {
    /// The field the caller named.
    pub(crate) field: &'a FieldId,
    /// Its metadata.
    pub(crate) meta: &'a FieldMeta,
    /// Where the value sits in the document.
    pub(crate) path: &'a [Segment<'a>],
    /// True when the stored value is a string.
    pub(crate) text: bool,
}

/// Writes `value` at `target` and returns the `Set` that restores the old
/// value. The checks run in this order: the field bounds under `policy`, the
/// ID tables (`Enforce` only), the stored value read, the stored value against
/// the storage bounds so the inverse applies, then the write. A failed call
/// leaves the document as it was.
pub(crate) fn set_value(document: &mut Document, tables: &GameTables, target: &Target<'_>, value: &Value, policy: CapPolicy) -> Result<Edit, EditError> {
    let Target { field, meta, path, text } = *target;
    meta.check(field, value, policy)?;
    if policy == CapPolicy::Enforce {
        check_tables(tables, meta, field, value)?;
    }
    let old = read_old(document, path, text, field)?;
    meta.check(field, &old, CapPolicy::StorageOnly)?;
    let scalar = match value {
        Value::Int(int) => Scalar::Int(*int),
        Value::Text(text) => Scalar::Text(text.clone()),
        Value::Bool(flag) => Scalar::Bool(*flag),
        // Unreachable: `meta.check` above rejects a kind the field's shape does not take.
        _ => return Err(EditError::ReadOnly { field: field.clone() }),
    };
    document.set(path, &scalar).map_err(|source| format_error(field, source))?;
    Ok(Edit::Set {
        field: field.clone(),
        value: old,
    })
}

/// A field with tables takes only ids that one of them lists. A
/// placeholder is an entry of its table, so the bare-hand placeholder
/// of a slot passes (`docs/formats/pixel-remaster.md#equipment-slots`).
fn check_tables(tables: &GameTables, meta: &FieldMeta, field: &FieldId, value: &Value) -> Result<(), EditError> {
    let Value::Int(id) = value else { return Ok(()) };
    if meta.tables.is_empty() {
        return Ok(());
    }
    let listed = u32::try_from(*id).is_ok_and(|id| meta.tables.iter().any(|kind| tables.entry(*kind, id).is_some()));
    if listed {
        Ok(())
    } else {
        Err(EditError::UnknownId { field: field.clone(), id: *id })
    }
}

/// The stored value, read the way `get` reads it.
fn read_old(document: &mut Document, path: &[Segment<'_>], text: bool, field: &FieldId) -> Result<Value, EditError> {
    let read = if text {
        read_text(document, path).map(Value::Text)
    } else {
        read_int(document, path).map(Value::Int)
    };
    read.map_err(|source| format_error(field, source))
}
