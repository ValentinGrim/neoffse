//! The private FF II slot files, decoded. Failure messages carry the label and
//! a field path, never a value, a length or a name
//! (`docs/architecture.md#test-policy`).

use neoffse_format_pr::document::Document;
use neoffse_format_pr::ff2::{Ff2Model, GAME};

use super::ff1_real::{RealSlot, slots_of};

/// The FF II private files named in `SLOT_FILES` that hold a `userData` key.
/// `None` after a skip report when no slot file is left.
pub(crate) fn slots() -> Option<Vec<RealSlot>> {
    slots_of("ff2")
}

/// A model over the decoded JSON of `slot`.
pub(crate) fn model(slot: &RealSlot) -> Ff2Model {
    let Ok(document) = Document::parse(&slot.json) else {
        panic!("{}: the decoded JSON does not parse", slot.label)
    };
    let Some(tables) = neoffse_data::tables_for(GAME) else {
        panic!("no FF II tables")
    };
    Ff2Model::new(document, tables)
}
