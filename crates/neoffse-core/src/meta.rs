//! Field metadata: what an editor needs to show and check one field.
//!
//! [`FieldMeta`] is the contract between a format and the app
//! (`docs/architecture.md#ui-architecture`). The view model derives the
//! widget from it: a non-empty
//! [`tables`](FieldMeta::tables) gives a searchable dropdown, non-empty
//! [`names`](FieldMeta::names) a dropdown beside the raw value, [`Shape::Int`]
//! a spin box clamped to the bounds with a warning above
//! [`cap`](FieldMeta::cap) (and an expert-mode switch when an edit returns
//! `EditError::AboveCap`), [`Shape::Bool`] a switch, [`Shape::Text`] a line
//! edit, and [`read_only`](FieldMeta::read_only) a label. Core defines no
//! widget type.

use crate::error::EditError;
use crate::field::{FieldId, Value, ValueKind};
use crate::kinds::{CapPolicy, TableKind};

/// A raw value and the name the game shows for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedValue {
    /// The raw value as the save stores it.
    pub value: i64,
    /// The name the game shows for it.
    pub name: String,
}

/// The values a field accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Shape {
    /// A flag.
    Bool,
    /// A string.
    Text,
    /// A whole number. `min` and `max` are the storage bounds, inclusive:
    /// the range the save can hold and the game reads back.
    Int {
        /// The lowest value the save can hold.
        min: i64,
        /// The highest value the save can hold.
        max: i64,
    },
}

impl Shape {
    /// The kind of value this shape accepts.
    #[must_use]
    pub fn kind(&self) -> ValueKind {
        match self {
            Shape::Bool => ValueKind::Bool,
            Shape::Text => ValueKind::Text,
            Shape::Int { .. } => ValueKind::Int,
        }
    }
}

/// What an editor needs to show and check one field.
///
/// The struct is `#[non_exhaustive]` so a later version adds a field.
/// [`FieldMeta::new`] plus assignment builds one:
///
/// ```
/// use neoffse_core::{FieldMeta, Shape, TableKind};
///
/// let mut meta = FieldMeta::new("Job", Shape::Int { min: 1, max: 12 });
/// meta.tables.push(TableKind::Job);
/// assert_eq!(meta.label, "Job");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldMeta {
    /// The text beside the field.
    pub label: String,
    /// The values the field accepts.
    pub shape: Shape,
    /// The field shows a value and accepts no edit.
    pub read_only: bool,
    /// The game's own cap, when it is lower than the storage bound.
    pub cap: Option<i64>,
    /// Named values for an `Int`; empty when none.
    pub names: Vec<NamedValue>,
    /// ID tables the `Int` references; empty when it references none.
    pub tables: Vec<TableKind>,
}

impl FieldMeta {
    /// A writable field with no cap, no names and no table.
    #[must_use]
    pub fn new(label: impl Into<String>, shape: Shape) -> Self {
        Self {
            label: label.into(),
            shape,
            read_only: false,
            cap: None,
            names: Vec::new(),
            tables: Vec::new(),
        }
    }

    /// Checks `value` against the shape and, under [`CapPolicy::Enforce`],
    /// against the cap.
    ///
    /// The order: `read_only`, then the kind of the value, then the storage
    /// bounds of an `Int`, then (only under `Enforce`) the cap. A value past
    /// both the bounds and the cap reports `OutOfRange`. The check ignores
    /// `names` and `tables`: named values label raw values and do not
    /// restrict them, and core holds no table (a format raises `UnknownId`).
    /// A shape with `min > max` is not validated at construction: every
    /// `Int` then fails `OutOfRange`.
    ///
    /// The check serves new edits. A format never validates an inverse with
    /// `Enforce`: a document receives `StorageOnly` for inverses.
    ///
    /// # Errors
    ///
    /// `ReadOnly`, `WrongType`, `OutOfRange` or `AboveCap`, each naming
    /// `field`.
    pub fn check(&self, field: &FieldId, value: &Value, policy: CapPolicy) -> Result<(), EditError> {
        if self.read_only {
            return Err(EditError::ReadOnly { field: field.clone() });
        }
        if value.kind() != self.shape.kind() {
            return Err(EditError::WrongType {
                field: field.clone(),
                expected: self.shape.kind(),
                found: value.kind(),
            });
        }
        let (Shape::Int { min, max }, Value::Int(number)) = (self.shape, value) else {
            return Ok(());
        };
        if *number < min || *number > max {
            return Err(EditError::OutOfRange {
                field: field.clone(),
                value: *number,
                min,
                max,
            });
        }
        if policy == CapPolicy::Enforce
            && let Some(cap) = self.cap
            && *number > cap
        {
            return Err(EditError::AboveCap {
                field: field.clone(),
                value: *number,
                cap,
            });
        }
        Ok(())
    }

    /// Whether the field has a cap and `value` is an `Int` above it.
    #[must_use]
    pub fn exceeds_cap(&self, value: &Value) -> bool {
        match (self.cap, value) {
            (Some(cap), Value::Int(number)) => *number > cap,
            _ => false,
        }
    }

    /// The name of the first [`NamedValue`] whose `value` matches.
    #[must_use]
    pub fn name_of(&self, value: i64) -> Option<&str> {
        self.names.iter().find(|named| named.value == value).map(|named| named.name.as_str())
    }
}
