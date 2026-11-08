//! What the six Pixel Remaster games share: the section names every slot
//! uses, the document paths of its three collections, and the reads that
//! find a row or a dictionary entry by key
//! (`docs/formats/pixel-remaster.md#container-shapes`,
//! `docs/formats/pixel-remaster.md#documents-inside-strings`).
//!
//! The layer also holds the character-field machinery: the field bindings,
//! the reads, the raw and game edits, the summary of a slot and the party
//! view. A game supplies its `Part` enum of character fields, its tables and
//! its rules, and calls this layer for the rest.
//!
//! A game plugs into [`SlotFormat`] and [`SlotDocument`] by implementing
//! [`SlotModel`] for its model, through the `impl_slot_model!` macro, then
//! names the pair with a type alias: `Ff1Format` is `SlotFormat<Ff1Model>`
//! and `Ff1Document` is `SlotDocument<Ff1Model>`. [`SlotFormat`] runs the
//! open pipeline: it decodes the container, parses the JSON text, applies
//! the game rule and checks the shape. The summary of a slot, the party
//! section and the inventory section of its view are the same for every game.
//!
//! The crate-private modules split the machinery as follows:
//!
//! - `bind` resolves a field id to gil, a character part or an item count,
//!   and defines the `CharacterPart` trait that a game's `Part` implements.
//! - `access` reads values, metas, locks and row keys, and applies the raw
//!   edits.
//! - `derived` reads and clamps the figures computed from a status row
//!   (`max_hp`, `max_mp`).
//! - `plan` checks a game edit, runs its plan and rolls it back as a whole
//!   when a step fails.
//! - `summary` and `view` build the slot summary and the shared sections of
//!   the view.
//! - `model` holds [`SlotModel`] and the `impl_slot_model!` macro, which
//!   writes the trait impl of a game model from its constants.

pub(crate) mod access;
pub(crate) mod bind;
pub(crate) mod derived;
mod document;
mod format;
pub(crate) mod items;
pub(crate) mod meta;
mod model;
pub mod names;
pub(crate) mod open;
pub(crate) mod paths;
pub(crate) mod plan;
pub(crate) mod read;
pub(crate) mod rules;
pub(crate) mod stat;
pub(crate) mod summary;
pub(crate) mod view;
pub(crate) mod write;

pub use document::SlotDocument;
pub use format::SlotFormat;
pub use model::SlotModel;
pub(crate) use model::impl_slot_model;
