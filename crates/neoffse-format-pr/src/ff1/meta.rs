//! The metadata of each declared field and collection, from the field table
//! and the ID tables and caps of `neoffse-data`.
//!
//! A meta describes a field (label, storage bounds, game cap, tables) and no
//! policy or game rule. It reads no document.

use neoffse_core::{CollectionMeta, FieldId, FieldMeta, NamedValue, RowField, Shape, TableKind, Value};
use neoffse_data::GameTables;

use super::binding::{Binding, Collection, List, Part, Stat};
use super::names;

/// Largest stored value of a number field. Neither the master data nor the
/// saves state a bound for these fields: `i32::MAX` is the clamp of the Go
/// editor and holds every counter of FF I
/// (`docs/formats/pixel-remaster.md#bounds-in-the-master-data`, "Storage
/// bound").
pub(super) const STORAGE_MAX: i64 = 2_147_483_647;

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

/// The tables that an `inventory` row key may name: items, weapons, shields,
/// armor, helmets and gloves share one `contentId` space
/// (`docs/formats/pixel-remaster.md#inventory`,
/// `docs/formats/pixel-remaster.md#id-spaces-and-tables`).
const INVENTORY_TABLES: [TableKind; 6] = [
    TableKind::Item,
    TableKind::Weapon,
    TableKind::Shield,
    TableKind::Armor,
    TableKind::Helmet,
    TableKind::Gloves,
];

fn slot(slot: u32) -> (&'static str, TableKind) {
    let index = usize::try_from(slot).ok().and_then(|number| number.checked_sub(1));
    index.and_then(|index| SLOTS.get(index)).copied().unwrap_or(("Slot", TableKind::Item))
}

/// An integer field from `min` to the storage bound, capped by the game
/// constant `bound` when the tables hold it.
fn int(label: impl Into<String>, min: i64, bound: Option<(&GameTables, &str)>) -> FieldMeta {
    let mut meta = FieldMeta::new(label, Shape::Int { min, max: STORAGE_MAX });
    meta.cap = bound.and_then(|(tables, key)| tables.bound(key));
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
        Binding::Gil => int("Gil", 0, capped("MAX_GIL")),
        Binding::Count { list, .. } => count_meta(list, tables),
        Binding::Character { part, .. } => match part {
            Part::Name => FieldMeta::new("Name", Shape::Text),
            Part::Job => {
                let mut meta = int("Job", 0, None);
                meta.tables = vec![TableKind::Job];
                meta
            }
            Part::Level => int("Level", 0, capped("MAX_LV")),
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

/// The `count` of an item row: 1 to the storage bound. `inventory` also takes
/// the `MAX_ITEM` cap. `key_items` takes none: `MAX_ITEM` caps the normal
/// inventory only, and the game interface keeps a key-item `count` at 1 by its
/// own rule (`docs/formats/pixel-remaster.md#inventory`,
/// `docs/formats/pixel-remaster.md#bounds-in-the-master-data`).
fn count_meta(list: List, tables: &GameTables) -> FieldMeta {
    let cap = (list == List::Inventory).then_some((tables, "MAX_ITEM"));
    int("Count", 1, cap)
}

/// The meta of a collection: row key, row fields and the insert and remove
/// flags. Rows keep the game's order, and the per-row locks live in the model.
pub(super) fn collection_meta(collection: Collection, tables: &GameTables) -> CollectionMeta {
    match collection {
        Collection::Character => {
            let key = int("Character", 1, None);
            let mut meta = CollectionMeta::new(FieldId::new(names::CHARACTER), "Characters", key);
            meta.insertable = false;
            meta.removable = false;
            meta
        }
        Collection::Items(list) => {
            let (label, key_label, key_tables) = match list {
                List::Inventory => ("Items", "Item", INVENTORY_TABLES.to_vec()),
                List::KeyItems => ("Key items", "Key item", vec![TableKind::KeyItem]),
            };
            let mut key = int(key_label, 1, None);
            key.tables = key_tables;
            let mut meta = CollectionMeta::new(FieldId::new(list.section()), label, key);
            meta.fields = vec![RowField::new(names::COUNT, count_meta(list, tables), Value::Int(1))];
            meta
        }
    }
}
