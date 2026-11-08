//! The raw write side: `Set` through the storage bounds and, under
//! `CapPolicy::Enforce`, the caps and the ID tables.
//!
//! The raw interface holds no game rule. A bare equipment `Set`, a count
//! below the wearers, a hand set to the shield placeholder, a gain past
//! `cap - base` and an edit of a placeholder row all pass when the value sits
//! inside the bounds of the policy.

use neoffse_core::{CapPolicy, Edit, EditError, FieldId, RowId, Value};

use super::Ff2Model;
use super::binding::Binding;
use super::keywords;
use crate::shared::items::Lists;
use crate::shared::write::{Target, set_value};

impl Ff2Model {
    /// Applies `edit` to the document and returns the edit that undoes it.
    ///
    /// A `Set` of a read-only field (`job`, `max_hp`, `max_mp`) is `ReadOnly`
    /// as soon as the field resolves. Otherwise it checks, in this order: the
    /// field resolves, its row and dictionary key exist, the value fits the
    /// storage bounds (both policies), the game cap (`Enforce` only) and the
    /// ID tables of the field (`Enforce` only), and the stored value fits the
    /// storage bounds too, so the inverse applies. The inverse is a `Set` of
    /// the old value that runs under `CapPolicy::StorageOnly`. A failed call
    /// leaves the document as it was.
    ///
    /// A `Set` of a spell level (`character.<id>.spell.<chain>`) is
    /// `ReadOnly` under both policies, before any document read: one `Set`
    /// cannot return the exact inverse of the places the level moves. The
    /// raw interface edits those places one by one (`ability.<i>.<part>`,
    /// `dictionary.<j>.<part>`, `addition_order.<k>`, `sort_order.<k>`), and
    /// [`apply_game`](Self::apply_game) writes them together.
    ///
    /// An `Insert` or `Remove` acts on a row of `inventory`, `key_items` or
    /// `keywords` and returns the `Remove` or `Insert` that restores the row
    /// at its place, bytes included. `Insert` checks the collection, the key bounds,
    /// the `count` value, under `Enforce` the ID tables of the list, then
    /// duplicates and the position. `Remove` refuses a row whose stored token
    /// is not the string token of the canonical `{"contentId":K,"count":N}`
    /// that the inverse `Insert` writes (`ReadOnly`: another text, order,
    /// spacing or escape) and a row whose inverse would not apply. Neither
    /// holds a game rule: a placeholder row and a worn row go like any other.
    /// A `character` row is `ReadOnly`.
    ///
    /// A keyword is a bare integer of `ownedKeyWaordList.target` with no
    /// field: `Insert` refuses a row value (`UnknownField`), a key below 1
    /// (`OutOfRange`), under `Enforce` a key the `Keyword` table does not list
    /// (`UnknownId`), a key already listed (`DuplicateRow`) and a position
    /// past the end. `Remove` takes the integer out under either policy
    /// (`MissingRow` for a key the list lacks) and returns an `Insert` at its
    /// old index.
    ///
    /// # Errors
    ///
    /// `UnknownField`, `MissingRow`, `WrongType`, `OutOfRange`, `AboveCap`,
    /// `UnknownId` (a job, an equipment id or a row key that no table lists,
    /// under `Enforce`), `DuplicateRow`, `PositionOutOfRange`,
    /// `NotACollection`, `ReadOnly` (a character row, a read-only or computed
    /// field, a row that cannot be restored byte for byte), and `Format` for
    /// a document of another shape or a stored value that is no integer.
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
        let meta = binding.meta(self.tables);
        // A read-only field refuses before any document read, so a slot of
        // another shape still answers `ReadOnly`.
        if meta.read_only {
            return Err(EditError::ReadOnly { field: field.clone() });
        }
        let path = binding.locate(&mut self.document, field)?;
        let target = Target {
            field,
            meta: &meta,
            path: &path,
            text: binding.is_text(),
        };
        set_value(&mut self.document, self.tables, &target, value, policy)
    }

    /// The item-list view of the model: the document and the tables.
    pub(super) fn lists(&mut self) -> Lists<'_> {
        Lists {
            document: &mut self.document,
            tables: self.tables,
        }
    }

    fn insert(&mut self, row: &RowId, position: Option<usize>, values: &[(&'static str, Value)], policy: CapPolicy) -> Result<Edit, EditError> {
        if keywords::owns(&row.collection()) {
            return keywords::insert(&mut self.document, self.tables, row, position, values, policy);
        }
        self.lists().insert(row, position, values, policy)
    }

    fn remove(&mut self, row: &RowId) -> Result<Edit, EditError> {
        if keywords::owns(&row.collection()) {
            return keywords::remove(&mut self.document, row);
        }
        self.lists().remove(row)
    }
}
