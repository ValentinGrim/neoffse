/// A failed read or edit of a [`Document`](super::Document).
///
/// Every `path` field holds a rendered location (see
/// [`render_path`](super::render_path)). For `MissingKey`, `DuplicateKey`,
/// `IndexOutOfRange` and `WrongKind` raised by a step, it is the object,
/// array or string the step started from. For `InvalidJson`, `NotADocument`,
/// `EmptyDocument`, `NumberOutOfRange`, `NonFiniteFloat` and `EndsInDoc` it
/// is the value itself.
///
/// The type derives `Debug` only: `serde_json::Error` is neither `Clone` nor
/// `PartialEq`. Tests match with `matches!`.
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// The text is not valid JSON, or a string holds an escape that no
    /// `String` can represent (a lone surrogate). The inner error names the
    /// line and column inside the text it parsed.
    #[error("invalid JSON at {path}: {source}")]
    InvalidJson {
        /// Rendered location of the value that failed to parse.
        path: String,
        /// The parser's error.
        source: serde_json::Error,
    },
    /// An object has no member with this key.
    #[error("no key `{key}` at {path}")]
    MissingKey {
        /// Rendered location of the object.
        path: String,
        /// The key that the path asked for.
        key: String,
    },
    /// An object holds this key more than once, so the path is ambiguous.
    #[error("key `{key}` occurs more than once at {path}")]
    DuplicateKey {
        /// Rendered location of the object.
        path: String,
        /// The repeated key.
        key: String,
    },
    /// An array is shorter than the index.
    #[error("index {index} is out of range at {path} (length {len})")]
    IndexOutOfRange {
        /// Rendered location of the array.
        path: String,
        /// The index that the path asked for.
        index: usize,
        /// The length of the array.
        len: usize,
    },
    /// The value is not of the kind that the step or the call needs.
    #[error("expected {expected} at {path}")]
    WrongKind {
        /// Rendered location of the value.
        path: String,
        /// `"object"`, `"array"`, `"string"` or `"scalar"`.
        expected: &'static str,
    },
    /// A `Doc` step met a string whose content is not a JSON object or
    /// array, or carries whitespace around one.
    #[error("the string at {path} does not hold a JSON object or array")]
    NotADocument {
        /// Rendered location of the string.
        path: String,
    },
    /// A `Doc` step met an empty string, the game's absent document
    /// (`docs/formats/pixel-remaster.md#documents-inside-strings`).
    #[error("the string at {path} is empty: no document inside")]
    EmptyDocument {
        /// Rendered location of the string.
        path: String,
    },
    /// An integer exceeds `i64` and `u64`, or a float overflows `f64`.
    #[error("the number at {path} does not fit i64, u64 or a finite f64")]
    NumberOutOfRange {
        /// Rendered location of the number.
        path: String,
    },
    /// A `Scalar::Float` holds NaN or an infinity, which JSON cannot spell.
    #[error("cannot write a NaN or infinite float at {path}")]
    NonFiniteFloat {
        /// Rendered location that the write targeted.
        path: String,
    },
    /// A write path ends in a `Doc` step. The caller replaces the string
    /// that holds the document instead.
    #[error("a path to set must not end in a document step ({path})")]
    EndsInDoc {
        /// Rendered location of the string that holds the document.
        path: String,
    },
    /// The path has more segments than `MAX_DEPTH`.
    #[error("a path holds more than {limit} segments")]
    TooDeep {
        /// The limit, `MAX_DEPTH`.
        limit: usize,
    },
}
