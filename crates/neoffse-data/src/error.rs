//! Errors of the table parser.

/// Why a set of table texts failed to parse.
///
/// `file` is the name the tool writes: `content.toml`, `abilities.toml`,
/// `jobs.toml`, `commands.toml`, `monsters.toml` or `bounds.toml`. `Display`
/// never repeats `source`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DataError {
    /// The text is no valid TOML, or its shape differs from the file contract.
    #[error("cannot parse {file}")]
    Toml {
        /// The file that failed.
        file: &'static str,
        /// The TOML error, with the line and column.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// A `kind` string the loader does not know.
    #[error("{file}: entry {id} has the unknown kind {kind:?}")]
    UnknownKind {
        /// The file that failed.
        file: &'static str,
        /// The id of the row.
        id: u32,
        /// The string found.
        kind: String,
    },
    /// An id below 1, repeated, or lower than the id before it.
    #[error("{file}: id {id} does not rise above the id before it (ids start at 1)")]
    IdOrder {
        /// The file that failed.
        file: &'static str,
        /// The offending id.
        id: u32,
    },
}
