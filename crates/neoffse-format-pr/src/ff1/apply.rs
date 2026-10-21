//! The raw write side: `Set` through the storage bounds and, under
//! `CapPolicy::Enforce`, the caps and the ID tables.
//!
//! The raw interface holds no game rule. A bare equipment `Set`, a count
//! below the wearers and an edit of a placeholder row all pass when the value
//! sits inside the bounds of the policy.

use neoffse_core::{CapPolicy, Edit, EditError, FieldId, FieldMeta, Value};

use super::binding::Binding;
use super::locate::{format_error, locate, read_int, read_text};
use super::meta;
use super::{Ff1Model, Scalar};
use crate::document::Segment;

impl Ff1Model {
    /// Applies `edit` to the document and returns the edit that undoes it.
    ///
    /// A `Set` of a read-only field (a spell slot) is `ReadOnly` as soon as
    /// the field resolves. Otherwise it checks, in this order: the field
    /// resolves, its row and dictionary key exist, the value fits the storage
    /// bounds (both policies), the game cap (`Enforce` only) and the ID
    /// tables of the field (`Enforce` only), and the stored value fits the
    /// storage bounds too, so the inverse applies. The inverse is a `Set` of
    /// the old value that runs under `CapPolicy::StorageOnly`. A failed call
    /// leaves the document as it was.
    ///
    /// An `Insert` or `Remove` acts on a row of `inventory` or `key_items`
    /// and returns the `Remove` or `Insert` that restores the row at its
    /// place, bytes included. `Insert` checks the collection, the key bounds,
    /// the `count` value, under `Enforce` the ID tables of the list, then
    /// duplicates and the position. `Remove` refuses a row whose stored token
    /// is not the string token of the canonical `{"contentId":K,"count":N}`
    /// that the inverse `Insert` writes (`ReadOnly`: another text, order,
    /// spacing or escape) and a row whose inverse would not apply. Neither
    /// holds a game rule: a placeholder row and a worn row go like any other.
    /// A `character` row is `ReadOnly`.
    ///
    /// # Errors
    ///
    /// `UnknownField`, `MissingRow`, `WrongType`, `OutOfRange`, `AboveCap`,
    /// `UnknownId` (a job, an equipment id or a row key that no table lists,
    /// under `Enforce`), `DuplicateRow`, `PositionOutOfRange`,
    /// `NotACollection`, `ReadOnly` (a character row, a spell slot, a row that
    /// cannot be restored byte for byte), `UnknownField` for a bad row value name, and
    /// `Format` for a document of another shape or a stored value that is no
    /// integer.
    pub fn apply(&mut self, edit: &Edit, policy: CapPolicy) -> Result<Edit, EditError> {
        match edit {
            Edit::Set { field, value } => self.set(field, value, policy),
            Edit::Insert { row, position, values } => self.insert(row, *position, values, policy),
            Edit::Remove { row } => self.remove(row),
            _ => Err(EditError::ReadOnly { field: edit.target().clone() }),
        }
    }

    fn set(&mut self, field: &FieldId, value: &Value, policy: CapPolicy) -> Result<Edit, EditError> {
        let binding = Binding::resolve(field)?;
        let meta = meta::field_meta(&binding, self.tables);
        // A read-only field refuses before any document read, so a slot of
        // another shape still answers `ReadOnly`.
        if meta.read_only {
            return Err(EditError::ReadOnly { field: field.clone() });
        }
        let path = locate(&mut self.document, &binding, field)?;
        meta.check(field, value, policy)?;
        if policy == CapPolicy::Enforce {
            self.check_tables(&meta, field, value)?;
        }
        let old = self.read_old(&binding, &path, field)?;
        meta.check(field, &old, CapPolicy::StorageOnly)?;
        let scalar = match value {
            Value::Int(int) => Scalar::Int(*int),
            Value::Text(text) => Scalar::Text(text.clone()),
            Value::Bool(flag) => Scalar::Bool(*flag),
            // Unreachable: `meta.check` above rejects a kind the field's shape does not take.
            _ => return Err(EditError::ReadOnly { field: field.clone() }),
        };
        self.document.set(&path, &scalar).map_err(|source| format_error(field, source))?;
        Ok(Edit::Set {
            field: field.clone(),
            value: old,
        })
    }

    /// A field with tables takes only ids that one of them lists. A
    /// placeholder is an entry of its table, so the bare-hand placeholder
    /// of a slot passes (`docs/formats/pixel-remaster.md#equipment-slots`).
    fn check_tables(&self, meta: &FieldMeta, field: &FieldId, value: &Value) -> Result<(), EditError> {
        let Value::Int(id) = value else { return Ok(()) };
        if meta.tables.is_empty() {
            return Ok(());
        }
        let listed = u32::try_from(*id).is_ok_and(|id| meta.tables.iter().any(|kind| self.tables.entry(*kind, id).is_some()));
        if listed {
            Ok(())
        } else {
            Err(EditError::UnknownId { field: field.clone(), id: *id })
        }
    }

    /// The stored value, read the way `get` reads it.
    fn read_old(&mut self, binding: &Binding, path: &[Segment<'_>], field: &FieldId) -> Result<Value, EditError> {
        let read = if binding.is_text() {
            read_text(&mut self.document, path).map(Value::Text)
        } else {
            read_int(&mut self.document, path).map(Value::Int)
        };
        read.map_err(|source| format_error(field, source))
    }
}
