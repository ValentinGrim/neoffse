//! The lock that tells which rows the game interface refuses to remove, and
//! the `Insert` and `Remove` entry points over the shared item-list code
//! (`crate::shared::items`).

use neoffse_core::{CapPolicy, Edit, EditError, RowId, Value};

use super::{Ff1Model, names};
use crate::shared::items::Lists;
use crate::shared::paths::Collection;

impl Ff1Model {
    /// The item-list view of the model: the document and the tables.
    fn lists(&mut self) -> Lists<'_> {
        Lists {
            document: &mut self.document,
            tables: self.tables,
        }
    }

    pub(super) fn insert(&mut self, row: &RowId, position: Option<usize>, values: &[(&'static str, Value)], policy: CapPolicy) -> Result<Edit, EditError> {
        self.lists().insert(row, position, values, policy)
    }

    pub(super) fn remove(&mut self, row: &RowId) -> Result<Edit, EditError> {
        self.lists().remove(row)
    }

    /// True when the game interface refuses to remove `row` whatever the
    /// values are: a `character` row (the collection is not removable), a
    /// placeholder row of `inventory`, a row whose id a character wears, or a
    /// row whose stored token is not canonical, whether by its text or by its
    /// escapes (`docs/formats/pixel-remaster.md#inventory`), or a canonical
    /// row whose stored `count` leaves the storage bound of a count (0, a
    /// negative number or above `i32::MAX`). `Remove` refuses such a row on
    /// both interfaces, because its inverse `Insert` could not store the
    /// count.
    ///
    /// It reads the document and writes nothing.
    ///
    /// # Errors
    ///
    /// `NotACollection` for a row of another path, `MissingRow` for a row the
    /// document does not hold (a character row included), `Format` when a read
    /// fails.
    pub fn row_locked(&mut self, row: &RowId) -> Result<bool, EditError> {
        let (meta, collection) = self.lists().row_collection(row)?;
        let index = self.lists().row_index(row, collection)?;
        let Collection::Items(list) = collection else {
            return Ok(true);
        };
        if self.is_placeholder(list, row.key()) || self.wearers_of(row.key(), row.id())? > 0 {
            return Ok(true);
        }
        let Some(count) = self.lists().canonical_row(row, list, index)? else {
            return Ok(true);
        };
        let values = [(names::COUNT, Value::Int(count))];
        Ok(meta.check_insert(row, &values, CapPolicy::StorageOnly).is_err())
    }
}
