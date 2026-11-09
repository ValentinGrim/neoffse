//! The path to each FF I character part, and the reads that need a
//! character's own layout (spell slots). Row and dictionary lookups live in
//! [`crate::shared::read`].

use neoffse_core::{EditError, FieldId, Value};

use super::binding::Part;
use crate::document::{Document, DocumentError, Segment};
use crate::shared::paths::{Path, element, with};
use crate::shared::read::{Located, entry_value, format_error, read_int, read_key, read_text, slot_path, wrong_kind};

/// Index of the element of the `abilitySlotDataList` array `target` whose
/// `level` is `wanted`, by value (`docs/formats/pixel-remaster.md#abilities`,
/// "FF I spell slots").
fn find_level(document: &mut Document, target: &[Segment<'static>], wanted: u32) -> Result<Option<usize>, DocumentError> {
    for index in 0..document.array_len(target)? {
        let level = with(element(target.to_vec(), index), &[Segment::Key("level")]);
        if read_int(document, &level)? == i64::from(wanted) {
            return Ok(Some(index));
        }
    }
    Ok(None)
}

/// The `abilityId` held by the spell slot value at `path`: 0 for `""`, the
/// id of the ability object otherwise. A number, a text that holds no object
/// with an integer `abilityId`, or an id outside `u32` is a `DocumentError`.
fn read_ability(document: &mut Document, path: &[Segment<'_>]) -> Result<u32, DocumentError> {
    if read_text(document, path)?.is_empty() {
        return Ok(0);
    }
    let id = [Segment::Doc, Segment::Key("abilityId")];
    let id_path: Vec<Segment<'_>> = path.iter().chain(&id).copied().collect();
    match read_key(document, &id_path) {
        // A slot object without `abilityId` is a bad shape, not an absent field.
        Err(DocumentError::MissingKey { .. }) => Err(wrong_kind(&id_path, "integer")),
        other => other,
    }
}

/// The ability id in the spell slot `part` of the character at `base`, as a
/// number.
pub(super) fn spell_ability(document: &mut Document, part: Part, field: &FieldId, base: Path) -> Result<Value, EditError> {
    let path = locate_part(document, base, part).map_err(|error| error.into_error(field))?;
    read_ability(document, &path)
        .map(|ability| Value::Int(i64::from(ability)))
        .map_err(|source| format_error(field, source))
}

pub(super) fn locate_part(document: &mut Document, base: Path, part: Part) -> Result<Path, Located> {
    let parameter = [Segment::Key("parameter"), Segment::Doc];
    let key = |name| [Segment::Key(name)];
    Ok(match part {
        Part::Name => with(base, &key("name")),
        Part::Job => with(base, &key("jobId")),
        Part::Exp => with(base, &key("currentExp")),
        Part::Level => with(with(base, &parameter), &key("addtionalLevel")),
        Part::Hp => with(with(base, &parameter), &key("currentHP")),
        Part::MaxHpGain => with(with(base, &parameter), &key("addtionalMaxHp")),
        Part::AccuracyGain => with(with(base, &parameter), &key("addtionalAccuracyRate")),
        Part::MagicEvasionGain => with(with(base, &parameter), &key("addtionalAbilityEvasionRate")),
        Part::Stat(stat) => with(with(base, &parameter), &key(stat.document_key())),
        Part::Charges(level) | Part::ChargesMaxGain(level) => {
            let list = if matches!(part, Part::Charges(_)) {
                "currentMpCountList"
            } else {
                "addtionalMaxMpCountList"
            };
            let dictionary = with(with(base, &parameter), &[Segment::Key(list), Segment::Doc]);
            entry_value(document, dictionary, level)?
        }
        // `locate` refuses a computed field before it gets here.
        Part::MaxHp | Part::NextLevel => return Err(Located::Absent),
        Part::Spell { level, slot } => {
            let target = with(base, &[Segment::Key("abilitySlotDataList"), Segment::Doc, Segment::Key("target")]);
            let index = find_level(document, &target, level)?.ok_or(Located::Absent)?;
            let dictionary = with(element(target, index), &[Segment::Key("slotInfo"), Segment::Doc]);
            entry_value(document, dictionary, slot)?
        }
        Part::Slot(slot) | Part::SlotCount(slot) => slot_path(document, base, slot, matches!(part, Part::SlotCount(_)))?,
    })
}
