//! Domain model shared by every save format, and the two traits a format
//! implements. No format, no file access, no UI.
//!
//! A format maps its save onto fields. A [`FieldId`] names a field, a
//! [`Value`] fills it, and a [`FieldMeta`] tells an editor how to show and
//! check it. An [`Edit`] sets a field or inserts or removes a row of a
//! collection. Applying an edit returns its inverse, which is how undo works,
//! and a [`ChangeSet`] applies the edits of one user action all or none.
//!
//! A [`SaveDocument`] edits through two interfaces over the same [`Edit`]:
//!
//! - [`apply`](SaveDocument::apply) is the raw interface. It checks the
//!   storage bounds of the field and, under [`CapPolicy::Enforce`], the
//!   game's cap and the ID tables. [`CapPolicy::StorageOnly`] accepts any
//!   value the save can hold: the expert mode of an app. Inverses and
//!   rollbacks always run under `StorageOnly`.
//! - [`apply_game`](SaveDocument::apply_game) edits the way the game would.
//!   It always enforces the caps and the game rules of the format, may expand
//!   one edit into several plain edits, and returns a [`GameChange`] with the
//!   edits applied and their inverses. [`row_locked`](SaveDocument::row_locked)
//!   and [`field_locked`](SaveDocument::field_locked) tell the UI which
//!   controls it refuses whatever the value.
//!
//! The modules:
//!
//! - [`collection`]: which collections accept `Insert` and `Remove`, and what a new row holds.
//! - [`edit`]: the three edit kinds, the change set that groups them and the result of a game-rule edit.
//! - [`error`]: the reasons an edit is refused or an operation fails.
//! - [`field`]: field and row identity, and the values an edit writes.
//! - [`files`]: named files, the detection probe and the claim of a format.
//! - [`mod@format`]: the `SaveFormat` and `SaveDocument` traits.
//! - [`game`]: the games whose saves NeoFFSE reads.
//! - [`kinds`]: capabilities, ID table kinds and the cap policy.
//! - [`meta`]: field metadata and its checks.
//! - [`view`]: the summary and the view the app reads.
//!
//! A document implements `apply`; [`ChangeSet::apply_with`] turns it into an
//! all-or-none change. A one-field document shows the contract:
//!
//! ```
//! use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FieldId, Value};
//!
//! let mut gil = 100;
//! let changes = ChangeSet::from_iter([Edit::set(FieldId::new("gil"), Value::Int(250))]);
//! let undo = changes.apply_with(CapPolicy::Enforce, |edit, _policy| match edit {
//!     Edit::Set { field, value: Value::Int(new) } => {
//!         let old = std::mem::replace(&mut gil, *new);
//!         Ok(Edit::set(field.clone(), Value::Int(old)))
//!     }
//!     _ => Err(EditError::ReadOnly { field: edit.target().clone() }),
//! })?;
//! assert_eq!(gil, 250);
//! assert_eq!(undo.len(), 1);
//! # Ok::<(), EditError>(())
//! ```

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
pub use edit::{ChangeSet, Edit, GameChange};
pub use error::{EditError, FormatError};
pub use field::{FieldId, Key, RowId, Value, ValueKind};
pub use files::{Confidence, Detection, FileProbe, PROBE_HEAD_LEN, SaveFiles};
pub use format::{SaveDocument, SaveFormat};
pub use game::Game;
pub use kinds::{CapPolicy, Capability, TableKind};
pub use meta::{FieldMeta, NamedValue, Shape};
pub use view::{Entity, FieldView, SaveSlot, SaveSummary, SaveView, Section};
