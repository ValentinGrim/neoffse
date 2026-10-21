//! The editor page: the pure functions that decide what widget a field gets,
//! what a typed input means, how an edit error becomes a notice and which
//! picker entries match a search, and the view model that binds them to an
//! open session.
//!
//! The pure functions hold no session and no window. Each reads the metadata
//! the format publishes (`FieldMeta`) or the game's tables and returns plain
//! data or a generated Slint type, so a test calls it without a screen.
//! [`build`] turns a `SaveView` into the Slint data, and [`EditorViewModel`]
//! sends each commit to the session as one edit. `leave` asks before a dirty
//! session closes. `overview` shapes the party cards; it is the one module
//! that names game fields. The Slint side lives in `ui/editor-types.slint` and
//! `ui/bridge.slint`.

mod field_row;
mod input;
mod kind;
mod layout;
mod leave;
mod model;
mod notice;
mod overview;
mod pick;

pub use input::{Input, value_for};
pub use kind::field_kind;
pub use layout::{Built, Locks, build};
pub use leave::{Leave, LeaveAnswer};
pub use model::EditorViewModel;
pub use notice::{notice_of, problem_notice};
pub use overview::overview;
pub use pick::{entry_name, picker_entries};
