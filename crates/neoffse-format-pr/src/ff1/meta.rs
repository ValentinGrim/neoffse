//! The metadata of each declared field and collection, from the field table
//! and the ID tables and caps of `neoffse-data`.
//!
//! A meta describes a field (label, storage bounds, game cap, tables) and no
//! policy or game rule. It reads no document.

use neoffse_core::{FieldMeta, NamedValue, Shape, TableKind};
use neoffse_data::GameTables;

use super::binding::{Binding, Part, Stat};
use crate::shared::meta::{count_meta, gil_meta, int};

/// The slot labels and their equipment tables, slot 1 to 5. The table of a
/// slot follows `parts_group_id` of the game's `armor.csv`
/// (`docs/formats/pixel-remaster.md#game-master-data`); the labels are the Go
/// editor's (`docs/formats/pixel-remaster.md#equipment-slots`).
const SLOTS: [(&str, TableKind); 5] = [
    ("Right hand", TableKind::Weapon),
    ("Left hand", TableKind::Shield),
    ("Body", TableKind::Armor),
    ("Head", TableKind::Helmet),
    ("Arms", TableKind::Gloves),
];

fn slot(slot: u32) -> (&'static str, TableKind) {
    let index = usize::try_from(slot).ok().and_then(|number| number.checked_sub(1));
    index.and_then(|index| SLOTS.get(index)).copied().unwrap_or(("Slot", TableKind::Item))
}

/// The level: 0 to `MAX_LV`. The game's cap is also the storage bound, so
/// expert mode refuses a level above it
/// (`docs/formats/pixel-remaster.md#characters`, "Level and Experience in NeoFFSE").
fn level_meta(tables: &GameTables) -> FieldMeta {
    let mut meta = int("Level", 0, Some((tables, "MAX_LV")));
    if let Some(max) = meta.cap {
        meta.shape = Shape::Int { min: 0, max };
    }
    meta
}

/// A computed field: an integer the editor shows and never sets.
fn derived_meta(label: &'static str, bound: Option<(&GameTables, &str)>) -> FieldMeta {
    let mut meta = int(label, 0, bound);
    meta.read_only = true;
    meta
}

fn stat_label(stat: Stat) -> &'static str {
    match stat {
        Stat::Power => "Power",
        Stat::Vitality => "Vitality",
        Stat::Agility => "Agility",
        Stat::Intelligence => "Intelligence",
        Stat::Spirit => "Spirit",
        Stat::Luck => "Luck",
        Stat::Magic => "Magic",
    }
}

/// The meta of the field that `binding` names.
pub(super) fn field_meta(binding: &Binding, tables: &GameTables) -> FieldMeta {
    let capped = |key| Some((tables, key));
    match *binding {
        Binding::Gil => gil_meta(tables),
        Binding::Count { list, .. } => count_meta(list, tables),
        Binding::Character { part, .. } => match part {
            Part::Name => FieldMeta::new("Name", Shape::Text),
            Part::Job => {
                let mut meta = int("Job", 0, None);
                meta.tables = vec![TableKind::Job];
                meta
            }
            Part::Level => level_meta(tables),
            Part::Exp => int("Experience", 0, capped("MAX_EXP")),
            Part::Hp => int("HP", 0, capped("MAX_HP")),
            Part::MaxHpGain => int("Max HP gain", 0, None),
            Part::AccuracyGain => int("Accuracy gain", 0, None),
            Part::MagicEvasionGain => int("Magic evasion gain", 0, None),
            Part::MaxHp => derived_meta("Max HP", capped("MAX_HP")),
            Part::NextLevel => derived_meta("Next level", None),
            Part::Stat(stat) => int(stat_label(stat), 0, stat.bound_key().map(|key| (tables, key))),
            Part::Charges(level) => int(format!("Charges, level {level}"), 0, capped("MAX_MAGICAL_TIMES")),
            Part::ChargesMaxGain(level) => int(format!("Max charges gain, level {level}"), 0, None),
            Part::Slot(number) => {
                let (label, table) = slot(number);
                let mut meta = int(label, 0, None);
                meta.tables = vec![table];
                meta
            }
            Part::SlotCount(number) => int(format!("Count in slot {number}"), 0, None),
            Part::Spell { level, slot } => spell_meta(level, slot),
        },
    }
}

/// A spell slot: the `abilityId` it holds, 0 for an empty slot. The slot is
/// read only (`docs/formats/pixel-remaster.md#abilities`, "FF I spell slots").
fn spell_meta(level: u32, slot: u32) -> FieldMeta {
    let mut meta = int(format!("Level {level}, slot {slot}"), 0, None);
    meta.read_only = true;
    meta.tables = vec![TableKind::Ability];
    meta.names = vec![NamedValue {
        value: 0,
        name: "Empty".to_owned(),
    }];
    meta
}
