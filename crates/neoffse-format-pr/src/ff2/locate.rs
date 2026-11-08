//! The path to each FF II character part. Row and dictionary lookups live in
//! [`crate::shared::read`].

use super::binding::Part;
use super::places;
use crate::document::{Document, Segment};
use crate::shared::paths::{Path, with};
use crate::shared::read::{Located, entry_value, slot_path};

pub(super) fn locate_part(document: &mut Document, base: Path, part: Part) -> Result<Path, Located> {
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
        // `locate` refuses a computed field and a spell level before it gets here.
        Part::MaxHp | Part::MaxMp | Part::Spell(_) => return Err(Located::Absent),
        Part::Place(place) => places::path(document, &base, place)?,
        Part::Slot(slot) | Part::SlotCount(slot) => slot_path(document, base, slot, matches!(part, Part::SlotCount(_)))?,
    })
}
