//! The path that reaches the value of a binding. Row and dictionary lookups
//! live in [`crate::shared::read`].

use neoffse_core::{EditError, FieldId};

use super::binding::{Binding, Part};
use crate::document::{Document, Segment};
use crate::shared::paths::{Path, gil_path, with};
use crate::shared::read::{Located, character_part, count_path, entry_value, slot_path};

/// The path to the value of `binding`. `MissingRow` for an absent row,
/// `UnknownField` for a dictionary without the key, `Format` for a read that
/// fails, `ReadOnly` for a computed field, which has no path.
pub(super) fn locate(document: &mut Document, binding: &Binding, field: &FieldId) -> Result<Path, EditError> {
    if binding.is_derived() {
        return Err(EditError::ReadOnly { field: field.clone() });
    }
    match *binding {
        Binding::Gil => Ok(gil_path()),
        Binding::Count { list, key } => count_path(document, list, key, field),
        Binding::Character { id, part } => character_part(document, id, field, |document, base| locate_part(document, base, part)),
    }
}

fn locate_part(document: &mut Document, base: Path, part: Part) -> Result<Path, Located> {
    let parameter = [Segment::Key("parameter"), Segment::Doc];
    let key = |name| [Segment::Key(name)];
    Ok(match part {
        Part::Name => with(base, &key("name")),
        Part::Job => with(base, &key("jobId")),
        Part::Hp => with(with(base, &parameter), &key("currentHP")),
        Part::Mp => with(with(base, &parameter), &key("currentMP")),
        Part::MaxHpGain => with(with(base, &parameter), &key("addtionalMaxHp")),
        Part::MaxMpGain => with(with(base, &parameter), &key("addtionalMaxMp")),
        Part::Stat(stat) => with(with(base, &parameter), &key(stat.document_key())),
        Part::Skill(skill) => {
            // The value of a skill key is an integer, not a string document.
            let dictionary = with(base, &[Segment::Key("skillLevelTargets"), Segment::Doc]);
            entry_value(document, dictionary, skill)?
        }
        // `locate` refuses a computed field before it gets here.
        Part::MaxHp | Part::MaxMp => return Err(Located::Absent),
        Part::Slot(slot) | Part::SlotCount(slot) => slot_path(document, base, slot, matches!(part, Part::SlotCount(_)))?,
    })
}
