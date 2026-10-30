//! What the six Pixel Remaster games share: the section names every slot
//! uses, the document paths of its three collections, and the reads that
//! find a row or a dictionary entry by key
//! (`docs/formats/pixel-remaster.md#container-shapes`,
//! `docs/formats/pixel-remaster.md#documents-inside-strings`).
//!
//! A game module keeps its own field vocabulary and rules and calls this
//! layer for the rest. It plugs into [`SlotFormat`] and [`SlotDocument`] by
//! implementing [`SlotModel`] for its model, then names the pair with a type
//! alias: `Ff1Format` is `SlotFormat<Ff1Model>` and `Ff1Document` is
//! `SlotDocument<Ff1Model>`. [`SlotFormat`] runs the open pipeline: it
//! decodes the container, parses the JSON text, applies the game rule and
//! checks the shape. The summary of a slot and the inventory section of its
//! view are the same for every game.

mod document;
mod format;
pub(crate) mod items;
pub(crate) mod meta;
mod model;
pub mod names;
pub(crate) mod open;
pub(crate) mod paths;
pub(crate) mod read;
pub(crate) mod summary;
pub(crate) mod view;
pub(crate) mod write;

pub use document::SlotDocument;
pub use format::SlotFormat;
pub use model::SlotModel;
