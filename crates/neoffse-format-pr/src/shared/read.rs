//! Reads that find a row or a dictionary entry and the errors they raise.
//!
//! Rows are found by the value of `id` or `contentId`, dictionary entries by
//! their key, never by position (`docs/formats/pixel-remaster.md#characters`,
//! `docs/formats/pixel-remaster.md#known-pitfalls`, item 22). Every lookup is
//! a read: it parses levels and never marks one dirty.

use neoffse_core::{EditError, FieldId, FormatError, RowId};

use super::paths::{Collection, Path, element, with};
use crate::document::{Document, DocumentError, Scalar, Segment, render_path};

/// The error of a failed document read, bound to the field that asked.
pub(crate) fn format_error(field: &FieldId, source: DocumentError) -> EditError {
    EditError::Format {
        field: field.clone(),
        source: Box::new(source),
    }
}

/// A failure while opening or reading a slot, as the error of the format.
pub(crate) fn decode_error(source: impl std::error::Error + Send + Sync + 'static) -> FormatError {
    FormatError::Decode { source: Box::new(source) }
}

/// The scalar at `path` as an integer. A float, a text or a `Uint` above
/// `i64::MAX` is `WrongKind { expected: "integer" }`.
pub(crate) fn read_int(document: &mut Document, path: &[Segment<'_>]) -> Result<i64, DocumentError> {
    match document.get(path)? {
        Scalar::Int(int) => Ok(int),
        Scalar::Uint(uint) => i64::try_from(uint).map_err(|_| wrong_kind(path, "integer")),
        _ => Err(wrong_kind(path, "integer")),
    }
}

/// The scalar at `path` as text; any other kind is `WrongKind { expected: "string" }`.
pub(crate) fn read_text(document: &mut Document, path: &[Segment<'_>]) -> Result<String, DocumentError> {
    match document.get(path)? {
        Scalar::Text(text) => Ok(text),
        _ => Err(wrong_kind(path, "string")),
    }
}

/// A `WrongKind` error at `path`.
pub(crate) fn wrong_kind(path: &[Segment<'_>], expected: &'static str) -> DocumentError {
    DocumentError::WrongKind {
        path: render_path(path),
        expected,
    }
}

/// The identifier of the row `key` (a `u32` by construction of the caller),
/// or a `WrongKind` when the stored value is no `u32`.
pub(crate) fn read_key(document: &mut Document, path: &[Segment<'_>]) -> Result<u32, DocumentError> {
    u32::try_from(read_int(document, path)?).map_err(|_| wrong_kind(path, "unsigned 32-bit integer"))
}

/// The `id` or `contentId` of each element of `collection`, in document order.
pub(crate) fn row_keys(document: &mut Document, collection: Collection) -> Result<Vec<u32>, DocumentError> {
    let target = collection.target();
    let len = document.array_len(&target)?;
    (0..len)
        .map(|index| {
            let key = Segment::Key(collection.row_key());
            read_key(document, &with(element(target.clone(), index), &[key]))
        })
        .collect()
}

/// Index of the first element of `collection` whose key is `wanted`. Keys are
/// read in order up to the match: an element whose key is no integer, before
/// the match, fails the call with `WrongKind` instead of being skipped.
pub(crate) fn find_row(document: &mut Document, collection: Collection, wanted: u32) -> Result<Option<usize>, DocumentError> {
    let target = collection.target();
    for index in 0..document.array_len(&target)? {
        let key = with(element(target.clone(), index), &[Segment::Key(collection.row_key())]);
        if read_int(document, &key)? == i64::from(wanted) {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// Index of the entry whose key is `wanted` in the dictionary at `dictionary`
/// (`docs/formats/pixel-remaster.md#container-shapes`).
pub(crate) fn find_entry(document: &mut Document, dictionary: &[Segment<'static>], wanted: u32) -> Result<Option<usize>, DocumentError> {
    let keys = with(dictionary.to_vec(), &[Segment::Key("keys")]);
    for index in 0..document.array_len(&keys)? {
        if read_int(document, &with(keys.clone(), &[Segment::Index(index)]))? == i64::from(wanted) {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// The `MissingRow` error of the row `key` in `collection`.
pub(crate) fn missing_row(collection: &'static str, key: u32) -> EditError {
    EditError::MissingRow {
        row: RowId::new(FieldId::new(collection), key).id().clone(),
    }
}

/// Why a path into a dictionary could not be built.
pub(crate) enum Located {
    /// The dictionary holds no entry for the key.
    Absent,
    /// A document read failed.
    Failed(DocumentError),
}

impl From<DocumentError> for Located {
    fn from(source: DocumentError) -> Self {
        Self::Failed(source)
    }
}

/// An entry of a dictionary with the given key, at `values[index]`.
pub(crate) fn entry_value(document: &mut Document, dictionary: Path, key: u32) -> Result<Path, Located> {
    let index = find_entry(document, &dictionary, key)?.ok_or(Located::Absent)?;
    Ok(with(dictionary, &[Segment::Key("values"), Segment::Index(index)]))
}
