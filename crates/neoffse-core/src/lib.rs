//! Domain model shared by every save format: games, characters, items,
//! typed edits with their inverse, and the field metadata the editors
//! read to pick a widget. No format, no file access, no UI.
//!
//! - [`collection`]: which collections accept `Insert` and `Remove`, and what a new row holds.
//! - [`edit`]: the three edit kinds and the change set that groups them.
//! - [`error`]: the reasons an edit is refused or an operation fails.
//! - [`field`]: field and row identity, and the values an edit writes.
//! - [`files`]: named files, the detection probe and the claim of a format.
//! - [`mod@format`]: the `SaveFormat` and `SaveDocument` traits.
//! - [`game`]: the games whose saves NeoFFSE reads.
//! - [`kinds`]: capabilities, ID table kinds and the cap policy.
//! - [`meta`]: field metadata and its checks.
//! - [`view`]: the summary and the view the app reads.

pub mod collection;
pub mod edit;
pub mod error;
pub mod field;
pub mod files;
pub mod format;
pub mod game;
pub mod kinds;
pub mod meta;
pub mod view;

pub use collection::{CollectionMeta, RowField};
pub use edit::{ChangeSet, Edit};
pub use error::{EditError, FormatError};
pub use field::{FieldId, Key, RowId, Value, ValueKind};
pub use files::{Confidence, Detection, FileProbe, PROBE_HEAD_LEN, SaveFiles};
pub use format::{SaveDocument, SaveFormat};
pub use game::Game;
pub use kinds::{CapPolicy, Capability, TableKind};
pub use meta::{FieldMeta, NamedValue, Shape};
pub use view::{Entity, FieldView, SaveSlot, SaveSummary, SaveView, Section};
