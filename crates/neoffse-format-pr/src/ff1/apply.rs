//! The raw write side: `Set` through the storage bounds and, under
//! `CapPolicy::Enforce`, the caps and the ID tables.
//!
//! The raw interface holds no game rule. A bare equipment `Set`, a count
//! below the wearers and an edit of a placeholder row all pass when the value
//! sits inside the bounds of the policy.

use neoffse_core::{CapPolicy, Edit, EditError};

use super::Ff1Model;
use super::binding::Part;
use crate::shared::access;

impl Ff1Model {
    /// Applies `edit` to the document and returns the edit that undoes it.
    ///
    /// A `Set` of a read-only field (a spell slot, `max_hp`, `next_level`) is `ReadOnly` as soon as
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
    /// `NotACollection`, `ReadOnly` (a character row, a spell slot, a computed field, a row that
    /// cannot be restored byte for byte), `UnknownField` for a bad row value name, and
    /// `Format` for a document of another shape or a stored value that is no
    /// integer.
    pub fn apply(&mut self, edit: &Edit, policy: CapPolicy) -> Result<Edit, EditError> {
        access::apply_raw::<Part, _>(&mut self.document, self.tables, edit, policy, |_, _| None)
    }
}
