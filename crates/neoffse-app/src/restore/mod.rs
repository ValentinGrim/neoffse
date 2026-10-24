//! The Restore page: the pure rules that turn a backup into a row and pick
//! the folder a restore writes into, and the view model that lists the
//! backups and runs the restore.
//!
//! A backup restores byte for byte and opens no format, so the page needs no
//! save logic (`docs/architecture.md#save-safety`, restore bullets). The
//! functions in `rows` and `target` read a backup's manifest and nothing
//! else; `model` keeps the list and the selection, `run` restores on a worker
//! (`docs/architecture.md#ui-architecture`). No row, notice or bridge value
//! carries a path.

mod model;
mod rows;
mod run;
mod target;

pub use model::RestoreViewModel;
pub use rows::{backup_row, backup_rows, reason_of};
pub use run::RestoreOutcome;
pub use target::{replaces_open_file, same_folder, target_of};
