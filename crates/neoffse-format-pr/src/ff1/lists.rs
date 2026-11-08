//! The lock that tells which rows the game interface refuses to remove.

use neoffse_core::{EditError, RowId};

use super::{Ff1Model, names};
use crate::shared::items::Lists;
use crate::shared::rules;

impl Ff1Model {
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
        rules::row_locked(&mut Lists::new(&mut self.document, self.tables), row, names::EQUIPMENT_SLOTS)
    }
}
