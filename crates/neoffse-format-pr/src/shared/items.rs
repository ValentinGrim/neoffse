//! `Insert` and `Remove` on the item lists.
//!
//! A row of `inventory` or `key_items` is a string document
//! `{"contentId":<K>,"count":<N>}` (`docs/formats/pixel-remaster.md#inventory`,
//! `docs/formats/pixel-remaster.md#container-shapes`). The model rebuilds a row
//! only in that compact form, so it removes only a row whose stored token is
//! the token the inverse `Insert` writes: same text and same escapes. The
//! inverse then restores the same bytes.

use neoffse_core::{CapPolicy, CollectionMeta, Edit, EditError, RowId, Value};
use neoffse_data::GameTables;

use super::meta::collection_meta;
use super::names;
use super::paths::{Collection, List};
use super::read::{find_row, format_error, read_text};
use crate::document::{Document, DocumentError, Scalar, Segment, string_token};

/// The canonical text of a row.
fn canonical(content_id: u32, count: i64) -> String {
    format!("{{\"contentId\":{content_id},\"count\":{count}}}")
}

/// The `count` of `text` when `text` has the shape of the canonical row of
/// `content_id`: the prefix `{"contentId":K,"count":`, an integer and the
/// closing brace. It does not decide canonicity: `canonical_row` compares the
/// stored token with the token of `canonical(..)`, which rejects other
/// spacing, another key order and other escapes. A negative count is
/// canonical (its text is what `canonical(..)` writes) and then fails the
/// storage check of `Remove` and `row_locked`.
fn canonical_count(text: &str, content_id: u32) -> Option<i64> {
    let head = format!("{{\"contentId\":{content_id},\"count\":");
    text.strip_prefix(&head)?.strip_suffix('}')?.parse().ok()
}

/// The document and the ID tables an item-list edit reads and writes.
pub(crate) struct Lists<'a> {
    /// The slot document.
    pub(crate) document: &'a mut Document,
    /// The ID tables of the game.
    pub(crate) tables: &'static GameTables,
}

impl Lists<'_> {
    /// The meta and the list kind of the collection of `row`: `NotACollection`
    /// for a path that is none of the three collections.
    pub(crate) fn row_collection(&self, row: &RowId) -> Result<(CollectionMeta, Collection), EditError> {
        let collection = row.collection();
        match Collection::of(&collection) {
            Some(kind) => Ok((collection_meta(kind, self.tables), kind)),
            None => Err(EditError::NotACollection { field: collection }),
        }
    }

    /// The index of `row` in its collection, or `MissingRow`.
    pub(crate) fn row_index(&mut self, row: &RowId, collection: Collection) -> Result<usize, EditError> {
        find_row(self.document, collection, row.key())
            .map_err(|source| format_error(row.id(), source))?
            .ok_or_else(|| EditError::MissingRow { row: row.id().clone() })
    }

    /// The `count` of the element `index` of `list` when its stored token is
    /// the one `insert` writes for the canonical row: the decoded text is
    /// canonical and the token carries no other escape than `serde_json`'s.
    pub(crate) fn canonical_row(&mut self, row: &RowId, list: List, index: usize) -> Result<Option<i64>, EditError> {
        let mut path = list.target();
        path.push(Segment::Index(index));
        let failed = |source| format_error(row.id(), source);
        let text = read_text(self.document, &path).map_err(failed)?;
        let Some(count) = canonical_count(&text, row.key()) else {
            return Ok(None);
        };
        let token = self.document.raw(&path).map_err(failed)?;
        Ok((*token == string_token(&canonical(row.key(), count))).then_some(count))
    }

    pub(crate) fn insert(&mut self, row: &RowId, position: Option<usize>, values: &[(&'static str, Value)], policy: CapPolicy) -> Result<Edit, EditError> {
        let (meta, collection) = self.row_collection(row)?;
        meta.check_insert(row, values, policy)?;
        // `check_insert` refuses a character row (`insertable` is false), so
        // only an item list reaches this point.
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
        if find_row(self.document, collection, row.key()).map_err(failed)?.is_some() {
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
        // `check_insert` already rejected a `count` that is no integer, and the
        // collection meta gives `count` an integer initial value.
        let Some(Value::Int(count)) = given.or(initial) else {
            return Err(EditError::ReadOnly { field: row.id().clone() });
        };
        let text = canonical(row.key(), *count);
        self.document.insert(&list.target(), index, &Scalar::Text(text)).map_err(failed)?;
        Ok(Edit::remove(row.clone()))
    }

    pub(crate) fn remove(&mut self, row: &RowId) -> Result<Edit, EditError> {
        let (meta, collection) = self.row_collection(row)?;
        meta.check_remove(row)?;
        // `check_remove` refuses a character row (`removable` is false), so
        // only an item list reaches this point.
        let Collection::Items(list) = collection else {
            return Err(EditError::ReadOnly { field: row.id().clone() });
        };
        let index = self.row_index(row, collection)?;
        let Some(count) = self.canonical_row(row, list, index)? else {
            return Err(EditError::ReadOnly { field: row.id().clone() });
        };
        let values = vec![(names::COUNT, Value::Int(count))];
        meta.check_insert(row, &values, CapPolicy::StorageOnly)?;
        self.document.remove(&list.target(), index).map_err(|source| format_error(row.id(), source))?;
        Ok(Edit::insert(row.clone(), Some(index), values))
    }
}
