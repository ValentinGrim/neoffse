//! Errors of the table parser.

/// Why a set of table texts failed to parse.
///
/// `file` is the name the tool writes: `content.toml`, `abilities.toml`,
/// `jobs.toml`, `commands.toml`, `monsters.toml`, `bounds.toml` or
/// `growth.toml`. `Display` never repeats `source`. The parser checks the file contract of
/// `data/pr/ff1/` row by row: the types (`Toml`), the kinds
/// (`UnknownKind`), the id order (`IdOrder`) and the value rules (`Rule`).
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
    /// A row that parses but breaks a rule of the file contract.
    #[error("{file}: {row} breaks a rule: {rule}")]
    Rule {
        /// The file that failed.
        file: &'static str,
        /// The row: `id <N>` for a table row, the key for a cap, `lv <N>` for a
        /// `growth.toml` level, the array name for an empty array.
        row: String,
        /// The rule, as a short phrase.
        rule: &'static str,
    },
}
