//! The metadata of each declared field and collection, from the field table
//! and the ID tables and caps of `neoffse-data`.
//!
//! A meta describes a field (label, storage bounds, game cap, tables) and no
//! policy or game rule. It reads no document.

use neoffse_core::{FieldMeta, Shape, TableKind};
use neoffse_data::GameTables;

use super::binding::{Binding, Part};
use crate::shared::meta::{count_meta, derived_meta, gil_meta, int};

/// The slot labels and the tables each slot takes. Slots 1 and 2 take a
/// weapon or a shield, slots 3 to 5 an armor, a helmet and gloves
/// (`docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`). The hand
/// labels are provisional until the game's own order is read (question 40).
const SLOTS: [(&str, &[TableKind]); 5] = [
    ("Hand 1", &[TableKind::Weapon, TableKind::Shield]),
    ("Hand 2", &[TableKind::Weapon, TableKind::Shield]),
    ("Body", &[TableKind::Armor]),
    ("Head", &[TableKind::Helmet]),
    ("Arms", &[TableKind::Gloves]),
];

/// The skill labels, keys 0 to 9
/// (`docs/formats/pixel-remaster.md#skill-levels-in-ff-ii`).
const SKILLS: [&str; 10] = [
    "Sword",
    "Knife",
    "Spear",
    "Axe",
    "Staff",
    "Bow",
    "Shield",
    "Bare hands",
    "Evasion",
    "Magic evasion",
];

/// The first skill key that reads `value / 100 + 1` for its level: evasion.
const FIRST_EVASION_KEY: u32 = 8;

/// The binding admits the slots 1 to 5 only, so the fallback row never shows.
fn slot(number: u32) -> (&'static str, &'static [TableKind]) {
    let index = usize::try_from(number).ok().and_then(|number| number.checked_sub(1));
    index.and_then(|index| SLOTS.get(index)).copied().unwrap_or(("Slot", &[TableKind::Item]))
}

/// The top value of a skill: weapons, shields and bare hands stop at the total
/// of `MAX_WEAPON_LEVEL`; evasion and magic evasion at 100 times their top
/// level minus 1, which is 1 500 in the shipped data
/// (`docs/formats/pixel-remaster.md#skill-levels-in-ff-ii`, "Top value"; the
/// evasion figure is derived from the code, not measured).
fn skill_cap(tables: &GameTables, key: u32) -> Option<i64> {
    // A cap key the tables lack, or a negative one, leaves the skill uncapped.
    let level = |name: &str| tables.bound(name).and_then(|level| u32::try_from(level).ok());
    if key < FIRST_EVASION_KEY {
        return tables.skill_total(level("MAX_WEAPON_LEVEL")?).map(i64::from);
    }
    let name = if key == FIRST_EVASION_KEY {
        "MAX_EVASION_TIMES_LEVEL"
    } else {
        "MAX_ABILITY_DEFENSE_TIMES_LEVEL"
    };
    Some(100 * (i64::from(level(name)?) - 1))
}

/// The meta of the field that `binding` names.
pub(super) fn field_meta(binding: &Binding, tables: &GameTables) -> FieldMeta {
    let capped = |key| Some((tables, key));
    match *binding {
        Binding::Gil => gil_meta(tables),
        Binding::Count { list, .. } => count_meta(list, tables),
        Binding::Character { part, .. } => match part {
            Part::Name => FieldMeta::new("Name", Shape::Text),
            // `jobId` names the person here, and no edit moves it
            // (`docs/formats/pixel-remaster.md#people-and-party-in-ff-ii`).
            Part::Job => {
                let mut meta = int("Person", 0, None);
                meta.read_only = true;
                meta.tables = vec![TableKind::Job];
                meta
            }
            Part::Hp => int("HP", 0, capped("MAX_HP")),
            Part::Mp => int("MP", 0, capped("MAX_MP")),
            Part::MaxHpGain => int("Max HP gain", 0, None),
            Part::MaxMpGain => int("Max MP gain", 0, None),
            Part::MaxHp => derived_meta("Max HP", capped("MAX_HP")),
            Part::MaxMp => derived_meta("Max MP", capped("MAX_MP")),
            // The cap of a stat gain is per character: the base is part of it.
            Part::Stat(stat) => int(format!("{} gain", stat.label()), 0, None),
            Part::Skill(key) => {
                // The binding admits the keys 0 to 9 only, so "Skill" never shows.
                let label = usize::try_from(key).ok().and_then(|key| SKILLS.get(key)).copied().unwrap_or("Skill");
                let mut meta = int(label, 0, None);
                meta.cap = skill_cap(tables, key);
                meta
            }
            Part::Slot(number) => {
                let (label, kinds) = slot(number);
                let mut meta = int(label, 0, None);
                meta.tables = kinds.to_vec();
                meta
            }
            Part::SlotCount(number) => int(format!("Count in slot {number}"), 0, None),
        },
    }
}
