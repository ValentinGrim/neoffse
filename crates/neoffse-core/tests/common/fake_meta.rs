//! Identities, metadata and scalar defaults of the fake format.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use neoffse_core::{CollectionMeta, FieldId, FieldMeta, NamedValue, RowField, RowId, Shape, TableKind, Value};

/// Row keys of `inventory` that name an entry of the (imaginary) item table.
pub(crate) const ITEMS: RangeInclusive<u32> = 1..=50;

pub(crate) fn inventory() -> FieldId {
    FieldId::new("inventory")
}

pub(crate) fn character() -> FieldId {
    FieldId::new("character")
}

pub(crate) fn gil() -> FieldId {
    FieldId::new("gil")
}

pub(crate) fn row(key: u32) -> RowId {
    RowId::new(inventory(), key)
}

pub(crate) fn hp(character_id: u32) -> FieldId {
    character().with_id(character_id).with_name("hp")
}

pub(crate) fn name(character_id: u32) -> FieldId {
    character().with_id(character_id).with_name("name")
}

pub(crate) fn job() -> FieldId {
    character().with_id(1).with_name("job")
}

pub(crate) fn locked() -> FieldId {
    character().with_id(1).with_name("locked")
}

pub(crate) fn count_field() -> RowField {
    RowField::new("count", FieldMeta::new("Count", Shape::Int { min: 1, max: 99 }), Value::Int(1))
}

pub(crate) fn inventory_meta() -> CollectionMeta {
    let mut key = FieldMeta::new("Item", Shape::Int { min: 1, max: 200 });
    key.tables.push(TableKind::Item);
    let mut meta = CollectionMeta::new(inventory(), "Inventory", key);
    meta.fields.push(count_field());
    meta
}

pub(crate) fn character_meta() -> CollectionMeta {
    let mut meta = CollectionMeta::new(character(), "Characters", FieldMeta::new("Character", Shape::Int { min: 1, max: 2 }));
    meta.insertable = false;
    meta.removable = false;
    meta
}

/// One scalar field: its metadata and its value.
#[derive(Debug, Clone)]
pub(crate) struct Slot {
    pub(crate) meta: FieldMeta,
    pub(crate) value: Value,
}

pub(crate) fn slot(label: &str, shape: Shape, value: Value) -> Slot {
    Slot {
        meta: FieldMeta::new(label, shape),
        value,
    }
}

pub(crate) fn scalar_slots() -> BTreeMap<FieldId, Slot> {
    let mut slots = BTreeMap::new();
    slots.insert(gil(), slot("Gil", Shape::Int { min: 0, max: 999_999 }, Value::Int(100)));
    for (id, hp_value, text) in [(1, 50, "Ada"), (2, 60, "Bo")] {
        let mut health = slot("HP", Shape::Int { min: 0, max: 9_999 }, Value::Int(hp_value));
        health.meta.cap = Some(999);
        slots.insert(hp(id), health);
        slots.insert(name(id), slot("Name", Shape::Text, Value::Text(text.into())));
    }
    let mut job_slot = slot("Job", Shape::Int { min: 1, max: 12 }, Value::Int(3));
    for (value, label) in [(1, "Warrior"), (2, "Thief")] {
        job_slot.meta.names.push(NamedValue { value, name: label.into() });
    }
    job_slot.meta.tables.push(TableKind::Job);
    slots.insert(job(), job_slot);
    let mut lock = slot("Locked", Shape::Int { min: 0, max: 255 }, Value::Int(7));
    lock.meta.read_only = true;
    slots.insert(locked(), lock);
    slots
}
