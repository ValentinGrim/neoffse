//! `Insert` and `Remove` on the item lists, and the lock that tells which
//! rows the game interface refuses to remove.
//!
//! A row of `inventory` or `key_items` is a string document
//! `{"contentId":<K>,"count":<N>}` (`docs/formats/pixel-remaster.md#inventory`,
//! `docs/formats/pixel-remaster.md#container-shapes`). The model rebuilds a row
//! only in that compact form, so it removes only a row that already has it:
//! the inverse `Insert` then restores the same bytes.

use neoffse_core::{CapPolicy, CollectionMeta, Edit, EditError, RowId, Value};

use super::binding::{Collection, List};
use super::locate::{find_row, format_error, read_text, wearers};
use super::{Ff1Model, Scalar, names};
use crate::document::{DocumentError, Segment};

/// The canonical text of a row.
fn canonical(content_id: u32, count: i64) -> String {
    format!("{{\"contentId\":{content_id},\"count\":{count}}}")
}

/// The `count` of `text` when `text` is exactly the canonical row of
/// `content_id`: no other key, no other order, plain decimal, no whitespace.
fn canonical_count(text: &str, content_id: u32) -> Option<i64> {
    let head = format!("{{\"contentId\":{content_id},\"count\":");
    let count: i64 = text.strip_prefix(&head)?.strip_suffix('}')?.parse().ok()?;
    (canonical(content_id, count) == text).then_some(count)
}

impl Ff1Model {
    /// The meta and the list kind of the collection of `row`: `NotACollection`
    /// for a path that is none of the three collections.
    fn row_collection(&self, row: &RowId) -> Result<(CollectionMeta, Collection), EditError> {
        let collection = row.collection();
        match (self.collection_meta(&collection), Collection::of(&collection)) {
            (Some(meta), Some(kind)) => Ok((meta, kind)),
            _ => Err(EditError::NotACollection { field: collection }),
        }
    }

    /// The index of `row` in its collection, or `MissingRow`.
    fn row_index(&mut self, row: &RowId, collection: Collection) -> Result<usize, EditError> {
        find_row(&mut self.document, collection, row.key())
            .map_err(|source| format_error(row.id(), source))?
            .ok_or_else(|| EditError::MissingRow { row: row.id().clone() })
    }

    /// The text of the element `index` of `list`.
    fn row_text(&mut self, row: &RowId, list: List, index: usize) -> Result<String, EditError> {
        let mut path = list.target();
        path.push(Segment::Index(index));
        read_text(&mut self.document, &path).map_err(|source| format_error(row.id(), source))
    }

    pub(super) fn insert(&mut self, row: &RowId, position: Option<usize>, values: &[(&'static str, Value)], policy: CapPolicy) -> Result<Edit, EditError> {
        let (meta, collection) = self.row_collection(row)?;
        meta.check_insert(row, values, policy)?;
        let Collection::Items(list) = collection else {
            return Err(EditError::ReadOnly { field: row.id().clone() });
        };
        if policy == CapPolicy::Enforce && !meta.key.tables.iter().any(|kind| self.tables.entry(*kind, row.key()).is_some()) {
            return Err(EditError::UnknownId {
                field: row.id().clone(),
                id: i64::from(row.key()),
            });
        }
        let failed = |source: DocumentError| format_error(row.id(), source);
        if find_row(&mut self.document, collection, row.key()).map_err(failed)?.is_some() {
            return Err(EditError::DuplicateRow { row: row.id().clone() });
        }
        let len = self.document.array_len(&list.target()).map_err(failed)?;
        let index = position.unwrap_or(len);
        if index > len {
            return Err(EditError::PositionOutOfRange {
                collection: row.collection(),
                position: index,
                len,
            });
        }
        let given = values.iter().find(|(name, _)| *name == names::COUNT).map(|(_, value)| value);
        let initial = meta.fields.iter().find(|field| field.name == names::COUNT).map(|field| &field.initial);
        let Some(Value::Int(count)) = given.or(initial) else {
            return Err(EditError::ReadOnly { field: row.id().clone() });
        };
        let text = canonical(row.key(), *count);
        self.document.insert(&list.target(), index, &Scalar::Text(text)).map_err(failed)?;
        Ok(Edit::remove(row.clone()))
    }

    pub(super) fn remove(&mut self, row: &RowId) -> Result<Edit, EditError> {
        let (meta, collection) = self.row_collection(row)?;
        meta.check_remove(row)?;
        let Collection::Items(list) = collection else {
            return Err(EditError::ReadOnly { field: row.id().clone() });
        };
        let index = self.row_index(row, collection)?;
        let text = self.row_text(row, list, index)?;
        let Some(count) = canonical_count(&text, row.key()) else {
            return Err(EditError::ReadOnly { field: row.id().clone() });
        };
        let values = vec![(names::COUNT, Value::Int(count))];
        meta.check_insert(row, &values, CapPolicy::StorageOnly)?;
        self.document.remove(&list.target(), index).map_err(|source| format_error(row.id(), source))?;
        Ok(Edit::insert(row.clone(), Some(index), values))
    }

    /// True when the game interface refuses to remove `row` whatever the
    /// values are: a `character` row (the collection is not removable), a
    /// placeholder row of `inventory`, a row whose id a character wears, or a
    /// row whose text is not canonical
    /// (`docs/formats/pixel-remaster.md#inventory`). A stored `count` of 0 is
    /// canonical and does not lock: the raw `Remove` refuses it with
    /// `OutOfRange`.
    ///
    /// It reads the document and writes nothing.
    ///
    /// # Errors
    ///
    /// `NotACollection` for a row of another path, `MissingRow` for a row the
    /// document does not hold (a character row included), `Format` when a read
    /// fails.
    pub fn row_locked(&mut self, row: &RowId) -> Result<bool, EditError> {
        let (_, collection) = self.row_collection(row)?;
        let index = self.row_index(row, collection)?;
        let Collection::Items(list) = collection else {
            return Ok(true);
        };
        let placeholder = list == List::Inventory && self.tables.content(row.key()).is_some_and(|entry| entry.placeholder);
        if placeholder {
            return Ok(true);
        }
        let worn = wearers(&mut self.document, row.key()).map_err(|source| format_error(row.id(), source))?;
        if worn > 0 {
            return Ok(true);
        }
        let text = self.row_text(row, list, index)?;
        Ok(canonical_count(&text, row.key()).is_none())
    }
}
