//! A minimal in-test document for the edit tests: scalar fields with
//! storage bounds and caps, and one ordered collection keyed by `u32`.
//! Every check goes through `FieldMeta::check` and `CollectionMeta`; only the
//! game rules (the ID table, duplicates, positions) live here.
//! Each test crate declares `mod common;`: Cargo does not build this file as
//! a test target.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::collections::BTreeMap;

pub(crate) mod fake;
pub(crate) mod fake_codec;
pub(crate) mod fake_meta;

use fake_meta::{ITEMS, count_field, inventory_meta};
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, FieldMeta, Key, RowId, Shape, Value};

/// The only collection of the model.
const INVENTORY: &str = "inventory";

/// One scalar field: its value and the metadata every edit goes through.
#[derive(Debug, Clone)]
struct Slot {
    value: Value,
    meta: FieldMeta,
}

impl Slot {
    fn int(value: i64, bounds: (i64, i64), cap: Option<i64>) -> Self {
        let mut meta = FieldMeta::new("Number", Shape::Int { min: bounds.0, max: bounds.1 });
        meta.cap = cap;
        Self {
            value: Value::Int(value),
            meta,
        }
    }

    fn other(value: Value) -> Self {
        let shape = match value {
            Value::Bool(_) => Shape::Bool,
            _ => Shape::Text,
        };
        Self {
            value,
            meta: FieldMeta::new("Other", shape),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Model {
    scalars: BTreeMap<FieldId, Slot>,
    rows: Vec<(u32, i64)>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Snapshot {
    pub(crate) scalars: BTreeMap<FieldId, Value>,
    pub(crate) rows: Vec<(u32, i64)>,
}

pub(crate) fn row(key: u32) -> RowId {
    RowId::new(FieldId::new(INVENTORY), key)
}

pub(crate) fn hp(character: u32) -> FieldId {
    FieldId::new("character").with_id(character).with_name("hp")
}

pub(crate) fn int(id: FieldId, v: i64) -> Edit {
    Edit::set(id, Value::Int(v))
}

pub(crate) fn text(id: FieldId, s: &str) -> Edit {
    Edit::set(id, Value::Text(s.to_string()))
}

pub(crate) fn insert(key: u32, position: Option<usize>, count: Option<i64>) -> Edit {
    let values = count.map(|n| ("count", Value::Int(n))).into_iter().collect();
    Edit::insert(row(key), position, values)
}

pub(crate) fn remove(key: u32) -> Edit {
    Edit::remove(row(key))
}

impl Model {
    pub(crate) fn sample() -> Model {
        let mut scalars = BTreeMap::new();
        scalars.insert(FieldId::new("gil"), Slot::int(0, (0, 999_999), None));
        for character in [1, 2] {
            scalars.insert(hp(character), Slot::int(100, (0, 9_999), Some(999)));
        }
        let name = |c: u32| FieldId::new("character").with_id(c).with_name("name");
        scalars.insert(name(1), Slot::other(Value::Text("Ada".into())));
        scalars.insert(name(2), Slot::other(Value::Text("Bo".into())));
        scalars.insert(FieldId::new("character").with_id(1).with_name("ready"), Slot::other(Value::Bool(false)));
        let mut locked = Slot::int(7, (0, 255), None);
        locked.meta.read_only = true;
        scalars.insert(FieldId::new("character").with_id(1).with_name("locked"), locked);
        Model {
            scalars,
            rows: vec![(3, 5), (7, 2), (9, 1)],
        }
    }

    pub(crate) fn snapshot(&self) -> Snapshot {
        Snapshot {
            scalars: self.scalars.iter().map(|(id, slot)| (id.clone(), slot.value.clone())).collect(),
            rows: self.rows.clone(),
        }
    }

    /// The scalar field ids.
    pub(crate) fn ids(&self) -> Vec<FieldId> {
        self.scalars.keys().cloned().collect()
    }

    /// The row keys, in collection order.
    pub(crate) fn rows(&self) -> Vec<u32> {
        self.rows.iter().map(|(key, _)| *key).collect()
    }

    pub(crate) fn value(&self, id: &FieldId) -> Option<&Value> {
        self.scalars.get(id).map(|slot| &slot.value)
    }

    fn index_of(&self, key: u32) -> Option<usize> {
        self.rows.iter().position(|(k, _)| *k == key)
    }

    /// Applies `edit` and returns its inverse. A refused edit changes nothing.
    pub(crate) fn apply(&mut self, edit: &Edit, policy: CapPolicy) -> Result<Edit, EditError> {
        match edit {
            Edit::Set { field, value } => self.set(field, value, policy),
            Edit::Insert { row, position, values } => self.insert(row, *position, values, policy),
            Edit::Remove { row } => self.remove(row),
            _ => Err(EditError::UnknownField { field: edit.target().clone() }),
        }
    }

    fn set(&mut self, field: &FieldId, value: &Value, policy: CapPolicy) -> Result<Edit, EditError> {
        if let [Key::Name(INVENTORY), Key::Id(key), Key::Name(name)] = field.keys() {
            let Some(index) = self.index_of(*key) else {
                return Err(EditError::MissingRow { row: row(*key).id().clone() });
            };
            if *name != "count" {
                return Err(EditError::UnknownField { field: field.clone() });
            }
            count_field().meta.check(field, value, policy)?;
            let (Value::Int(count), Some(slot)) = (value, self.rows.get_mut(index)) else {
                return Err(EditError::UnknownField { field: field.clone() });
            };
            let old = std::mem::replace(&mut slot.1, *count);
            return Ok(Edit::set(field.clone(), Value::Int(old)));
        }
        let Some(slot) = self.scalars.get_mut(field) else {
            return Err(EditError::UnknownField { field: field.clone() });
        };
        slot.meta.check(field, value, policy)?;
        let old = std::mem::replace(&mut slot.value, value.clone());
        Ok(Edit::set(field.clone(), old))
    }

    fn insert(&mut self, id: &RowId, position: Option<usize>, values: &[(&'static str, Value)], policy: CapPolicy) -> Result<Edit, EditError> {
        let collection = id.collection();
        inventory_meta().check_insert(id, values, policy)?;
        if policy == CapPolicy::Enforce && !ITEMS.contains(&id.key()) {
            return Err(EditError::UnknownId {
                field: id.id().clone(),
                id: i64::from(id.key()),
            });
        }
        if self.index_of(id.key()).is_some() {
            return Err(EditError::DuplicateRow { row: id.id().clone() });
        }
        let len = self.rows.len();
        let position = position.unwrap_or(len);
        if position > len {
            return Err(EditError::PositionOutOfRange { collection, position, len });
        }
        let count = values
            .iter()
            .find_map(|(_, value)| if let Value::Int(n) = value { Some(*n) } else { None })
            .unwrap_or(1);
        self.rows.insert(position, (id.key(), count));
        Ok(Edit::remove(id.clone()))
    }

    fn remove(&mut self, id: &RowId) -> Result<Edit, EditError> {
        inventory_meta().check_remove(id)?;
        let Some(index) = self.index_of(id.key()) else {
            return Err(EditError::MissingRow { row: id.id().clone() });
        };
        let (_, count) = self.rows.remove(index);
        Ok(Edit::insert(id.clone(), Some(index), vec![("count", Value::Int(count))]))
    }
}
