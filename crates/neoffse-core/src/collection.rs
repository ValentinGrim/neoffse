//! Collection metadata: which collections accept `Insert` and `Remove`, and
//! what a new row looks like.
//!
//! The UI reads the two flags to show or hide the add and remove controls.
//! [`CollectionMeta::key`] gives the bounds and the table kinds for the
//! dropdown of the "add" control, and [`CollectionMeta::fields`] gives one
//! input per row field. A format reuses [`CollectionMeta::check_insert`] and
//! [`CollectionMeta::check_remove`] instead of rewriting the checks. Row keys
//! are `u32` (`RowId`).

use crate::error::EditError;
use crate::field::{FieldId, RowId, Value};
use crate::kinds::CapPolicy;
use crate::meta::FieldMeta;

/// One field of a new row: its name under the row, its metadata and its
/// initial value.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RowField {
    /// The field name under the row: `count` for `inventory.7.count`.
    pub name: &'static str,
    /// What the editor needs to show and check the field.
    pub meta: FieldMeta,
    /// The value a new row takes when the `Insert` leaves the field out. It
    /// must satisfy `meta`; nothing validates it.
    pub initial: Value,
}

impl RowField {
    /// A row field from its three parts.
    #[must_use]
    pub fn new(name: &'static str, meta: FieldMeta, initial: Value) -> Self {
        Self { name, meta, initial }
    }
}

/// What the UI needs to know about a collection of rows.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CollectionMeta {
    /// The collection's path, the `RowId::collection` of its rows.
    pub id: FieldId,
    /// The text beside the collection.
    pub label: String,
    /// The collection accepts `Edit::Insert`.
    pub insertable: bool,
    /// The collection accepts `Edit::Remove`.
    pub removable: bool,
    /// The row key as a field: an `Int` shape with bounds, and the ID tables
    /// the key comes from (an inventory `contentId`).
    pub key: FieldMeta,
    /// The fields of a new row, in display order.
    pub fields: Vec<RowField>,
}

impl CollectionMeta {
    /// An insertable, removable collection with no row fields.
    #[must_use]
    pub fn new(id: FieldId, label: impl Into<String>, key: FieldMeta) -> Self {
        Self {
            id,
            label: label.into(),
            insertable: true,
            removable: true,
            key,
            fields: Vec::new(),
        }
    }

    /// Checks an `Insert` of `row` with `values` against the collection.
    ///
    /// The order: the row's collection, the `insertable` flag, the key
    /// against [`key`](CollectionMeta::key) (its errors name the row id),
    /// then each `(name, value)` in turn: a name that is no [`RowField`] or
    /// that repeats an earlier one gives `UnknownField`, otherwise the value
    /// goes through that field's `FieldMeta::check`. Fields left out of
    /// `values` take [`RowField::initial`], which the format applies.
    ///
    /// The function knows no row, no free position and no ID table: the
    /// format raises `DuplicateRow`, `MissingRow`, `PositionOutOfRange` and
    /// `UnknownId`.
    ///
    /// # Errors
    ///
    /// `NotACollection` for a row of another collection, `ReadOnly` when the
    /// collection refuses `Insert`, `UnknownField` for a bad or repeated
    /// name, and what `FieldMeta::check` returns for the key and the values.
    pub fn check_insert(&self, row: &RowId, values: &[(&'static str, Value)], policy: CapPolicy) -> Result<(), EditError> {
        self.check_collection(row)?;
        if !self.insertable {
            return Err(EditError::ReadOnly { field: self.id.clone() });
        }
        self.key.check(row.id(), &Value::Int(i64::from(row.key())), policy)?;
        for (index, (name, value)) in values.iter().enumerate() {
            let repeated = values.iter().take(index).any(|(earlier, _)| earlier == name);
            let template = self.fields.iter().find(|field| field.name == *name);
            match template {
                Some(field) if !repeated => field.meta.check(&row.field(name), value, policy)?,
                _ => return Err(EditError::UnknownField { field: row.field(name) }),
            }
        }
        Ok(())
    }

    /// Checks a `Remove` of `row` against the collection. The function knows
    /// no row: the format raises `MissingRow`.
    ///
    /// # Errors
    ///
    /// `NotACollection` for a row of another collection, `ReadOnly` when the
    /// collection refuses `Remove`.
    pub fn check_remove(&self, row: &RowId) -> Result<(), EditError> {
        self.check_collection(row)?;
        if !self.removable {
            return Err(EditError::ReadOnly { field: self.id.clone() });
        }
        Ok(())
    }

    fn check_collection(&self, row: &RowId) -> Result<(), EditError> {
        let collection = row.collection();
        if collection == self.id {
            Ok(())
        } else {
            Err(EditError::NotACollection { field: collection })
        }
    }
}
