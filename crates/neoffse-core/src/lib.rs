//! Domain model shared by every save format: games, characters, items,
//! typed edits with their inverse, and the field metadata the editors
//! read to pick a widget. No format, no file access, no UI.
//!
//! - [`game`]: the games whose saves NeoFFSE reads.
//! - [`field`]: field and row identity, and the values an edit writes.
//! - [`edit`]: the three edit kinds and the change set that groups them.
//! - [`error`]: the reasons an edit is refused or fails.
//! - [`kinds`]: capabilities, ID table kinds and the cap policy.

pub mod edit;
pub mod error;
pub mod field;
pub mod game;
pub mod kinds;

pub use edit::{ChangeSet, Edit};
pub use error::EditError;
pub use field::{FieldId, Key, RowId, Value, ValueKind};
pub use game::Game;
pub use kinds::{CapPolicy, Capability, TableKind};
