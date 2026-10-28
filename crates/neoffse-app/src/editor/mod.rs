//! The editor page: the pure functions that decide what widget a field gets,
//! what a typed input means, how an edit error becomes a notice and which
//! picker entries match a search, and the view model that binds them to an
//! open session.
//!
//! The pure functions hold no session and no window. Each reads the metadata
//! the format publishes (`FieldMeta`) or the game's tables and returns plain
//! data or a generated Slint type, so a test calls it without a screen.
//! [`build`] turns a `SaveView` into the Slint data, and [`EditorViewModel`]
//! sends each commit to the session as one edit. `history` counts the unsaved
//! changes and replays undo and redo, `save` runs the save pipeline on a
//! worker thread. `leave` asks before a dirty session closes. `growth` adds
//! the level-up notice to an edit that raised levels. `review` shapes the
//! change list of the review dialog and the time of a backup. `overview`
//! shapes the party cards; it is the one module
//! that names game fields. The Slint side lives in
//! `ui/types/editor.slint` and `ui/screens/editor/bridge.slint`.

mod edit;
mod field_row;
mod growth;
mod history;
mod input;
mod kind;
mod layout;
mod leave;
mod model;
mod notice;
mod overview;
mod pick;
mod review;
mod save;

pub use history::History;
pub use input::{Input, value_for};
pub use kind::field_kind;
pub use layout::{Built, Locks, build};
pub use leave::{Leave, LeaveAnswer};
pub use model::EditorViewModel;
pub use notice::{growth_notice, notice_of, problem_notice, reload_failed_notice, save_failed_notice, saved_notice};
pub use overview::{card_of_page, levels_gained, overview};
pub use pick::{entry_name, picker_entries};
pub use review::{review_rows, stamp_of};
pub use save::SaveOutcome;
