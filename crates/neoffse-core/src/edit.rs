//! The three edit kinds and the change set that groups them.
//!
//! A document applies an [`Edit`] and returns its inverse, which is how undo
//! works. [`ChangeSet::apply_with`] holds the one implementation of the
//! all-or-none rule.

use crate::error::EditError;
use crate::field::{FieldId, RowId, Value};
use crate::kinds::CapPolicy;

/// One change to a save.
///
/// A document answers an edit with its inverse, which undoes it. Redo is the
/// original edit applied again. The inverse of an inverse can differ from the
/// original in `position` (an `Insert` that appended comes back as
/// `Insert { position: Some(index) }`) and never in effect.
///
/// A later version may add kinds: a format matches with a wildcard arm and
/// answers an unknown kind with an error of its own.
///
/// ```
/// use neoffse_core::{Edit, FieldId, RowId, Value};
///
/// let gil = Edit::set(FieldId::new("gil"), Value::Int(500));
/// let row = RowId::new(FieldId::new("inventory"), 7);
/// let add = Edit::insert(row.clone(), None, vec![("count", Value::Int(2))]);
/// assert_eq!(add.target(), row.id());
/// assert_eq!(gil.target(), &FieldId::new("gil"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Edit {
    /// Sets a field to a value. The field takes the value and the inverse is
    /// a `Set` of the previous value. A path that runs through an absent row
    /// gives `EditError::MissingRow`.
    Set {
        /// The field to write.
        field: FieldId,
        /// The value to store.
        value: Value,
    },
    /// Adds a row to a collection.
    ///
    /// The row joins at `position` (`None`: at the end; `Some(i)` with
    /// `i <= len`: before the row now at `i`). `values` are initial field
    /// values by field name; the collection's initial value fills each field
    /// left out. The inverse is `Remove { row }`.
    ///
    /// A path that is no collection gives `NotACollection`; a collection that
    /// refuses the kind gives `ReadOnly`; a key already present gives
    /// `DuplicateRow`; a `position` past the end gives `PositionOutOfRange`;
    /// a key that is no entry of the referenced ID table gives `UnknownId`.
    Insert {
        /// The row to add.
        row: RowId,
        /// The index the row takes; `None` appends.
        position: Option<usize>,
        /// Initial field values by field name under the row.
        values: Vec<(&'static str, Value)>,
    },
    /// Removes a row from a collection.
    ///
    /// The inverse is an `Insert` with the row, `position: Some(old index)`
    /// and `values` holding every field the document models for that row, so
    /// applying it puts the row back at its place with its values and the
    /// order is the one before the `Remove`. A format that keeps data a row
    /// holds beyond its modeled fields restores that data too: it stashes the
    /// removed row in the document under the row key. An absent row gives
    /// `MissingRow`.
    Remove {
        /// The row to remove.
        row: RowId,
    },
}

impl Edit {
    /// A [`Edit::Set`].
    #[must_use]
    pub fn set(field: FieldId, value: Value) -> Self {
        Edit::Set { field, value }
    }

    /// A [`Edit::Insert`].
    #[must_use]
    pub fn insert(row: RowId, position: Option<usize>, values: Vec<(&'static str, Value)>) -> Self {
        Edit::Insert { row, position, values }
    }

    /// A [`Edit::Remove`].
    #[must_use]
    pub fn remove(row: RowId) -> Self {
        Edit::Remove { row }
    }

    /// The field or the row the edit addresses: `field`, or the row's `id()`.
    #[must_use]
    pub fn target(&self) -> &FieldId {
        match self {
            Edit::Set { field, .. } => field,
            Edit::Insert { row, .. } | Edit::Remove { row } => row.id(),
        }
    }
}

/// The edits of one user action, in the order the user made them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChangeSet {
    edits: Vec<Edit>,
}

impl ChangeSet {
    /// An empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an edit after the ones already in the set.
    pub fn push(&mut self, edit: Edit) {
        self.edits.push(edit);
    }

    /// The edits, in the order they apply.
    #[must_use]
    pub fn edits(&self) -> &[Edit] {
        &self.edits
    }

    /// The number of edits.
    #[must_use]
    pub fn len(&self) -> usize {
        self.edits.len()
    }

    /// Whether the set holds no edit.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.edits.is_empty()
    }

    /// Applies every edit through `apply` with `policy`; all or none. `apply`
    /// is the document's `apply(&Edit, CapPolicy) -> Result<Edit, EditError>`.
    ///
    /// The edits go in order, and `apply` returns the inverse of each. On
    /// success the result holds those inverses in reverse order: a caller
    /// applies that set, and any redo, with `CapPolicy::StorageOnly`.
    ///
    /// On the first error, the inverses already collected undo the applied
    /// edits, newest first, through `apply` with `CapPolicy::StorageOnly`
    /// whatever `policy` says: an inverse restores a value the document held,
    /// and that value may sit above the game cap. The original error is the
    /// result.
    ///
    /// With a well-behaved `apply`, the document holds every change after
    /// `Ok`, and after `Err` (other than `RollbackFailed`) it holds what it
    /// held before the call, rows in the same order. An empty set returns an
    /// empty set without calling `apply`.
    ///
    /// # Errors
    ///
    /// The error of the first edit `apply` refuses. When an undo call fails
    /// too, the undo stops and the result is `EditError::RollbackFailed`: the
    /// document is in an unknown state and the caller reloads it.
    pub fn apply_with<F>(&self, policy: CapPolicy, mut apply: F) -> Result<ChangeSet, EditError>
    where
        F: FnMut(&Edit, CapPolicy) -> Result<Edit, EditError>,
    {
        let mut inverses = Vec::with_capacity(self.edits.len());
        for edit in &self.edits {
            match apply(edit, policy) {
                Ok(inverse) => inverses.push(inverse),
                Err(error) => return Err(roll_back(&inverses, error, &mut apply)),
            }
        }
        inverses.reverse();
        Ok(ChangeSet { edits: inverses })
    }
}

/// What a game-rule edit did, as plain edits.
///
/// A document with game rules may expand one user edit into several plain
/// edits (equipping a weapon also touches the inventory). Both sets apply
/// through [`SaveDocument::apply_all`](crate::SaveDocument::apply_all) under
/// `CapPolicy::StorageOnly`, which holds no game rule, so neither can fail on
/// one: `forward` is the redo and `inverse` the undo.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct GameChange {
    /// The plain edits, in the order they applied.
    pub forward: ChangeSet,
    /// The inverses of `forward`, in reverse order.
    pub inverse: ChangeSet,
}

impl GameChange {
    /// A change from its redo and undo sets.
    #[must_use]
    pub fn new(forward: ChangeSet, inverse: ChangeSet) -> Self {
        Self { forward, inverse }
    }
}

/// Undoes `inverses`, newest first. Returns `error`, or `RollbackFailed` at
/// the first undo that fails.
fn roll_back<F>(inverses: &[Edit], error: EditError, apply: &mut F) -> EditError
where
    F: FnMut(&Edit, CapPolicy) -> Result<Edit, EditError>,
{
    for inverse in inverses.iter().rev() {
        if let Err(rollback) = apply(inverse, CapPolicy::StorageOnly) {
            return EditError::RollbackFailed {
                field: inverse.target().clone(),
                source: Box::new(error),
                rollback: Box::new(rollback),
            };
        }
    }
    error
}

impl FromIterator<Edit> for ChangeSet {
    fn from_iter<I: IntoIterator<Item = Edit>>(iter: I) -> Self {
        Self {
            edits: iter.into_iter().collect(),
        }
    }
}
