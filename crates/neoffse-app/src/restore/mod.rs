//! The Restore page's pure rules: how a backup becomes a row, and which
//! folder a restore writes into.
//!
//! A backup restores byte for byte and opens no format, so the page needs no
//! save logic (`docs/architecture.md#save-safety`, restore bullets). The
//! functions here read a backup's manifest and nothing else; the view model
//! and the glue sit in later modules (`docs/architecture.md#ui-architecture`).
//! No row, notice or bridge value carries a path.

mod rows;
mod target;

pub use rows::{backup_row, backup_rows, reason_of};
pub use target::{replaces_open_file, same_folder, target_of};
