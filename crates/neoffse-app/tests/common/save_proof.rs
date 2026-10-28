//! Compares a saved FF I slot with a reference run of the same two edits.
//! A message names the slot position and a field id, never a value: the slots
//! are personal data.

use neoffse::core::{FieldId, Key, SaveView, Value};
use neoffse::data::GameTables;
use neoffse::pr::ff1::names;

/// The stored fields that one level step rolls: the HP gain and the six stats
/// with Magic, in the order of the growth row (strength, vitality, agility,
/// intelligence, spirit, magic, luck).
const ROLLED: [&str; 8] = [
    names::MAX_HP_GAIN,
    names::POWER,
    names::VITALITY,
    names::AGILITY,
    names::INTELLIGENCE,
    names::SPIRIT,
    names::MAGIC,
    names::LUCK,
];

/// The HP bonus of a successful roll runs from 20 to 24 (`BONUS_RANGE` of the
/// FF I growth step).
const BONUS: (i64, i64) = (20, 24);

/// What the level step reads: the character, its job, base vitality and
/// vitality before the step, and the level it reaches.
pub(crate) struct Step<'a> {
    pub(crate) tables: &'a GameTables,
    pub(crate) character: u32,
    pub(crate) job: u32,
    pub(crate) base_vitality: u32,
    pub(crate) vitality: i64,
    pub(crate) new_level: u32,
}

/// Every field of `view` with its value, in traversal order.
pub(crate) fn flat(view: &SaveView) -> Vec<(FieldId, Value)> {
    view.sections
        .iter()
        .flat_map(|section| &section.entities)
        .flat_map(|entity| &entity.fields)
        .map(|field| (field.id.clone(), field.value.clone()))
        .collect()
}

fn int(value: &Value) -> Option<i64> {
    match value {
        Value::Int(number) => Some(*number),
        _ => None,
    }
}

/// The rolled name in `id`, if it is one of the fields of `ROLLED` or the
/// derived Max HP that follows the HP gain.
fn rolled(id: &FieldId) -> Option<&'static str> {
    id.keys().iter().find_map(|key| match key {
        Key::Name(name) if ROLLED.contains(name) || *name == names::MAX_HP => Some(*name),
        _ => None,
    })
}

/// The gain that one step allows for `name`, as `(low, high)`.
fn allowed(step: &Step<'_>, name: &str) -> Option<(i64, i64)> {
    let row = step.tables.growth_step(step.job, step.new_level)?;
    if name == names::MAX_HP_GAIN {
        let hp = (i64::from(step.base_vitality) + step.vitality).max(0) / 4 + 1;
        let forced = if row.strong_hp { BONUS.0 } else { 0 };
        return Some((hp + forced, hp + BONUS.1));
    }
    let column = i64::from(*row.stats.get(ROLLED.iter().skip(1).position(|rolled| *rolled == name)?)?);
    Some((column, column.max(1)))
}

/// `saved` and `reference` hold the same fields with the same values, but for
/// the rolled ones. A rolled field equals its value in `before`, except the
/// stored ones of the character of `step`: each of those moved from `before`
/// by an amount the growth rule allows. Returns the number of fields checked
/// against the rule.
pub(crate) fn assert_same(slot: usize, saved: &SaveView, reference: &SaveView, before: &SaveView, step: Option<&Step<'_>>) -> usize {
    let (saved, reference, before) = (flat(saved), flat(reference), flat(before));
    assert!(saved.len() == reference.len(), "slot {slot}: the saved view and the reference differ in size");
    assert!(saved.len() == before.len(), "slot {slot}: the saved view and the original differ in size");
    let mut checked = 0;
    for (((id, value), (other, expected)), (_, old)) in saved.iter().zip(&reference).zip(&before) {
        assert!(id == other, "slot {slot}: the field order differs at {id}");
        let Some(name) = rolled(id) else {
            assert!(value == expected, "slot {slot}: {id} differs from the reference");
            continue;
        };
        let moved = step.filter(|step| id.keys().get(1) == Some(&Key::Id(step.character)));
        let Some(step) = moved else {
            assert!(value == old, "slot {slot}: {id} changed outside the edited character");
            continue;
        };
        if name == names::MAX_HP {
            continue;
        }
        let (Some(now), Some(then)) = (int(value), int(old)) else {
            panic!("slot {slot}: {id} is not an integer")
        };
        let Some((low, high)) = allowed(step, name) else {
            panic!("slot {slot}: no growth row for {id}")
        };
        assert!((then + low..=then + high).contains(&now), "slot {slot}: {id} grew outside the rule");
        checked += 1;
    }
    checked
}
