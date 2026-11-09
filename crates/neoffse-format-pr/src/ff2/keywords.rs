//! The keywords: `userData.ownedKeyWaordList.target`, a flat list of integers
//! in acquisition order, each the content id of a type-6 row
//! (`docs/formats/pixel-remaster.md#keywords-in-ff-ii`).
//!
//! A row is a bare integer and has no field, so the collection is separate from
//! the item lists. The raw interface inserts and removes any listed key, the
//! game interface appends one and refuses every removal: the game adds a
//! keyword through `AddKeyWord` and has no method that removes one, so a
//! removal is an expert-mode edit of NeoFFSE whose effect in game is unproven
//! [I].

use neoffse_core::{CapPolicy, CollectionMeta, Edit, EditError, FieldId, Key, RowId, TableKind, Value};
use neoffse_data::GameTables;

use super::names;
use crate::document::{Document, DocumentError, Scalar, Segment};
use crate::shared::meta::int;
use crate::shared::paths::Path;
use crate::shared::read::{format_error, read_key};

/// Path to `userData.ownedKeyWaordList.target` (the game's spelling).
fn target() -> Path {
    vec![
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("ownedKeyWaordList"),
        Segment::Doc,
        Segment::Key("target"),
    ]
}

/// True when `collection` is the keyword collection.
pub(super) fn owns(collection: &FieldId) -> bool {
    matches!(collection.keys(), [Key::Name(names::KEYWORDS)])
}

/// The keywords in document order. An element that is no `u32` is a
/// `WrongKind` error.
pub(super) fn rows(document: &mut Document) -> Result<Vec<u32>, DocumentError> {
    let target = target();
    (0..document.array_len(&target)?)
        .map(|index| {
            let mut path = target.clone();
            path.push(Segment::Index(index));
            read_key(document, &path)
        })
        .collect()
}

/// The meta of the collection: the key is a content id of the `Keyword`
/// table, and a row holds no field.
pub(super) fn meta() -> CollectionMeta {
    let mut key = int("Keyword", 1, None);
    key.tables = vec![TableKind::Keyword];
    CollectionMeta::new(FieldId::new(names::KEYWORDS), "Keywords", key)
}

/// The index of `row` in the list, `MissingRow` when it is absent.
fn index_of(document: &mut Document, row: &RowId) -> Result<usize, EditError> {
    let keys = rows(document).map_err(|source| format_error(row.id(), source))?;
    keys.iter()
        .position(|key| *key == row.key())
        .ok_or_else(|| EditError::MissingRow { row: row.id().clone() })
}

/// The raw `Insert`: checks the key against the meta, under `Enforce` against
/// the `Keyword` table, then duplicates and the position. It returns the
/// `Remove` that undoes it.
pub(super) fn insert(
    document: &mut Document,
    tables: &GameTables,
    row: &RowId,
    position: Option<usize>,
    values: &[(&'static str, Value)],
    policy: CapPolicy,
) -> Result<Edit, EditError> {
    meta().check_insert(row, values, policy)?;
    if policy == CapPolicy::Enforce && tables.entry(TableKind::Keyword, row.key()).is_none() {
        return Err(EditError::UnknownId {
            field: row.id().clone(),
            id: i64::from(row.key()),
        });
    }
    let failed = |source: DocumentError| format_error(row.id(), source);
    let keys = rows(document).map_err(failed)?;
    if keys.contains(&row.key()) {
        return Err(EditError::DuplicateRow { row: row.id().clone() });
    }
    // `None` appends, as `AddKeyWord` does.
    let index = position.unwrap_or(keys.len());
    if index > keys.len() {
        return Err(EditError::PositionOutOfRange {
            collection: row.collection(),
            position: index,
            len: keys.len(),
        });
    }
    document.insert(&target(), index, &Scalar::Int(i64::from(row.key()))).map_err(failed)?;
    Ok(Edit::remove(row.clone()))
}

/// The raw `Remove`, under either policy: it takes the element out and returns
/// the `Insert` at its old index. A key the meta would refuse to insert
/// (below 1) stays, because the inverse could not apply.
pub(super) fn remove(document: &mut Document, row: &RowId) -> Result<Edit, EditError> {
    let collection = meta();
    collection.check_remove(row)?;
    let index = index_of(document, row)?;
    collection.check_insert(row, &[], CapPolicy::StorageOnly)?;
    document.remove(&target(), index).map_err(|source| format_error(row.id(), source))?;
    Ok(Edit::insert(row.clone(), Some(index), Vec::new()))
}

/// The game rule of an `Insert`: `AddKeyWord` appends, so a position other
/// than the end is `PositionOutOfRange`.
pub(super) fn check_game_insert(document: &mut Document, row: &RowId, position: Option<usize>) -> Result<(), EditError> {
    let Some(position) = position else { return Ok(()) };
    let len = rows(document).map_err(|source| format_error(row.id(), source))?.len();
    if position != len {
        return Err(EditError::PositionOutOfRange {
            collection: row.collection(),
            position,
            len,
        });
    }
    Ok(())
}

/// Every keyword row is locked in the game interface: no game path removes
/// one. `MissingRow` for a key the list lacks.
pub(super) fn row_locked(document: &mut Document, row: &RowId) -> Result<bool, EditError> {
    index_of(document, row)?;
    Ok(true)
}
