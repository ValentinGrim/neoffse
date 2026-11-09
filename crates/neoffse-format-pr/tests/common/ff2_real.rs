//! The private FF II slot files, decoded. Failure messages carry the label and
//! a field path, never a value, a length or a name
//! (`docs/architecture.md#test-policy`).

use super::real_slots::{RealSlot, slots_of};

/// The FF II private files named in `SLOT_FILES` that hold a `userData` key.
/// `None` after a skip report when no slot file is left.
pub(crate) fn slots() -> Option<Vec<RealSlot>> {
    slots_of("ff2")
}
