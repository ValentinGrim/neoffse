//! The editor page's pure functions: what widget a field gets, what a typed
//! input means, how an edit error becomes a notice, which picker entries
//! match a search.
//!
//! Nothing here holds a session or a window. Each function reads the
//! metadata the format publishes (`FieldMeta`) or the game's tables and
//! returns plain data or a generated Slint type, so a test calls it without
//! a screen. The Slint side lives in `ui/editor-types.slint` and
//! `ui/bridge.slint`.

mod input;
mod kind;
mod notice;
mod pick;

pub use input::{Input, value_for};
pub use kind::field_kind;
pub use notice::{notice_of, problem_notice};
pub use pick::{entry_name, picker_entries};
