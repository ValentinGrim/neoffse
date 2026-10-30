//! What the six Pixel Remaster games share: the section names every slot
//! uses, the document paths of its three collections, and the reads that
//! find a row or a dictionary entry by key
//! (`docs/formats/pixel-remaster.md#container-shapes`,
//! `docs/formats/pixel-remaster.md#documents-inside-strings`).
//!
//! A game module keeps its own field vocabulary and rules and calls this
//! layer for the rest.

pub(crate) mod meta;
pub mod names;
pub(crate) mod paths;
pub(crate) mod read;
