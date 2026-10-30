//! The path that reaches the value of a binding, and the FF I reads that
//! need a character's own layout (spell slots, equipment wearers). Row and
//! dictionary lookups live in [`crate::shared::read`].

use neoffse_core::{EditError, FieldId};

use super::binding::{Binding, Part};
use super::names;
use crate::document::{Document, DocumentError, Segment};
use crate::shared::paths::{Collection, Path, characters_target, element, with};
use crate::shared::read::{Located, entry_value, find_row, format_error, missing_row, read_int, read_key, read_text, wrong_kind};

/// The path to the value of `binding`. `MissingRow` for an absent row,
/// `UnknownField` for a dictionary without the key, `Format` for a read that
/// fails, `ReadOnly` for a computed field, which has no path.
pub(super) fn locate(document: &mut Document, binding: &Binding, field: &FieldId) -> Result<Path, EditError> {
    if binding.is_derived() {
        return Err(EditError::ReadOnly { field: field.clone() });
    }
    let failed = |source| format_error(field, source);
    match *binding {
        Binding::Gil => Ok(vec![Segment::Key("userData"), Segment::Doc, Segment::Key("owendGil")]),
        Binding::Count { list, key } => {
            let index = find_row(document, Collection::Items(list), key)
                .map_err(failed)?
                .ok_or_else(|| missing_row(list.section(), key))?;
            Ok(with(element(list.target(), index), &[Segment::Key("count")]))
        }
        Binding::Character { id, part } => {
            let index = find_row(document, Collection::Character, id)
                .map_err(failed)?
                .ok_or_else(|| missing_row(names::CHARACTER, id))?;
            let base = element(characters_target(), index);
            locate_part(document, base, part).map_err(|error| match error {
                Located::Absent => EditError::UnknownField { field: field.clone() },
                Located::Failed(source) => format_error(field, source),
            })
        }
    }
}

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
pub(super) fn read_ability(document: &mut Document, path: &[Segment<'_>]) -> Result<u32, DocumentError> {
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

fn locate_part(document: &mut Document, base: Path, part: Part) -> Result<Path, Located> {
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
        Part::Slot(slot) | Part::SlotCount(slot) => {
            let dictionary = with(base, &[Segment::Key("equipmentList"), Segment::Doc]);
            let value = entry_value(document, dictionary, slot)?;
            let name = if matches!(part, Part::Slot(_)) { "contentId" } else { "count" };
            with(value, &[Segment::Doc, Segment::Key(name)])
        }
    })
}

/// The number of equipment slots, over all characters, that hold
/// `content_id` (`docs/formats/pixel-remaster.md#inventory`: an equipped id
/// appears in the inventory with a count at least this number).
pub(super) fn wearers(document: &mut Document, content_id: u32) -> Result<usize, DocumentError> {
    let target = characters_target();
    let mut count = 0;
    for index in 0..document.array_len(&target)? {
        let dictionary = with(element(target.clone(), index), &[Segment::Key("equipmentList"), Segment::Doc]);
        let keys = with(dictionary.clone(), &[Segment::Key("keys")]);
        for entry in 0..document.array_len(&keys)? {
            let slot = read_key(document, &with(keys.clone(), &[Segment::Index(entry)]))?;
            if !(1..=names::EQUIPMENT_SLOTS).contains(&slot) {
                continue;
            }
            let value = [Segment::Key("values"), Segment::Index(entry), Segment::Doc, Segment::Key("contentId")];
            if read_key(document, &with(dictionary.clone(), &value))? == content_id {
                count += 1;
            }
        }
    }
    Ok(count)
}
