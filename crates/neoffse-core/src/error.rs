//! The reasons an edit is refused or fails.

use crate::field::{FieldId, ValueKind};

/// A refused or failed edit.
///
/// The type derives `Debug` only: `Format` and `RollbackFailed` box other
/// errors. Tests match with `matches!`. `Display` never repeats `source`
/// (`docs/architecture.md#errors`). A later version may add variants, so a
/// `match` needs a wildcard arm.
///
/// A refused edit leaves the document as it was.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum EditError {
    /// The format has no field at this path.
    #[error("unknown field {field}")]
    UnknownField {
        /// The path the edit named.
        field: FieldId,
    },
    /// The field exists and does not accept edits.
    #[error("field {field} is read-only")]
    ReadOnly {
        /// The path the edit named.
        field: FieldId,
    },
    /// The value is of another kind than the field holds.
    #[error("field {field} takes {expected}, not {found}")]
    WrongType {
        /// The path the edit named.
        field: FieldId,
        /// The kind the field holds.
        expected: ValueKind,
        /// The kind the edit carried.
        found: ValueKind,
    },
    /// The value breaks the storage bound of the field. A value past both the
    /// storage bound and the game cap reports this variant: the storage bound
    /// is checked first.
    #[error("{value} is outside {min}..={max} for field {field}")]
    OutOfRange {
        /// The path the edit named.
        field: FieldId,
        /// The value the edit carried.
        value: i64,
        /// The lowest value the save can hold, inclusive.
        min: i64,
        /// The highest value the save can hold, inclusive.
        max: i64,
    },
    /// The value breaks the game cap under `CapPolicy::Enforce`. The app reads
    /// this variant to offer expert mode; no `CapPolicy::StorageOnly` edit
    /// returns it.
    #[error("{value} is above the game cap {cap} for field {field}")]
    AboveCap {
        /// The path the edit named.
        field: FieldId,
        /// The value the edit carried.
        value: i64,
        /// The highest value the game allows, inclusive.
        cap: i64,
    },
    /// The value or row key is no entry of the ID table the field references.
    /// Core holds no table, so a format raises this variant.
    #[error("{id} names no entry for field {field}")]
    UnknownId {
        /// The path the edit named: the field, or the row for an `Insert`.
        field: FieldId,
        /// The value or key that names no entry.
        id: i64,
    },
    /// An `Insert` named a key the collection already holds.
    #[error("row {row} already exists")]
    DuplicateRow {
        /// The full id of the row, as `RowId::id` renders it.
        row: FieldId,
    },
    /// The row is absent from its collection.
    #[error("row {row} does not exist")]
    MissingRow {
        /// The full id of the row, as `RowId::id` renders it.
        row: FieldId,
    },
    /// An `Insert` or `Remove` named a path that is no collection.
    #[error("{field} is not a collection")]
    NotACollection {
        /// The path the edit named, or the collection path of its row.
        field: FieldId,
    },
    /// An `Insert` asked for a position past the end of the collection.
    #[error("position {position} is past the end of {collection} (length {len})")]
    PositionOutOfRange {
        /// The path of the collection.
        collection: FieldId,
        /// The position the edit asked for.
        position: usize,
        /// The number of rows the collection holds.
        len: usize,
    },
    /// The document layer failed. The failure is the source.
    #[error("cannot apply the edit to {field}")]
    Format {
        /// The path the edit named.
        field: FieldId,
        /// The document layer's error, a `DocumentError` for Pixel Remaster.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A change set failed and its rollback failed too. The document is in an
    /// unknown state: the caller reloads it. `Display` prints neither inner
    /// error.
    #[error("rollback failed at {field} after another edit failed")]
    RollbackFailed {
        /// The target of the inverse edit that failed to apply.
        field: FieldId,
        /// The error that stopped the change set.
        source: Box<EditError>,
        /// The error of the undo call.
        rollback: Box<EditError>,
    },
}

/// A failed format operation on files or documents.
///
/// A format crate converts its own errors into this type and keeps them as
/// the source (`docs/architecture.md#errors`). `Display` never repeats the
/// source. The type derives `Debug` only, since it boxes other errors, and
/// is `Error + Send + Sync + 'static`. It holds file names, which carry no
/// personal value, and no path. A later version may add variants, so a
/// `match` needs a wildcard arm.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FormatError {
    /// A file name that is not a plain file name: empty, `.`, `..`, or one
    /// holding `/`, `\`, `:` or a NUL byte.
    #[error("invalid file name {name:?}")]
    InvalidFileName {
        /// The name as given.
        name: String,
    },
    /// A file name already present in the `SaveFiles`, compared byte for byte.
    #[error("the file {name:?} appears twice")]
    DuplicateFile {
        /// The repeated name.
        name: String,
    },
    /// A file the format needs is absent from the `SaveFiles`.
    #[error("the save has no file {name:?}")]
    MissingFile {
        /// The name of the absent file.
        name: String,
    },
    /// The format cannot read the files. The format's own error is the source.
    #[error("cannot read the save")]
    Decode {
        /// The format's error: a `CodecError`, a `DocumentError`, and so on.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// The format cannot write the document. The format's own error is the
    /// source.
    #[error("cannot write the save")]
    Encode {
        /// The format's error: a `CodecError`, a `DocumentError`, and so on.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// The decoded file differs from the in-memory document.
    #[error("the file {name:?} does not match the document")]
    VerifyMismatch {
        /// The name of the first file that differs.
        name: String,
    },
}
