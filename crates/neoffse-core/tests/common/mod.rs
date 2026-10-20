//! A minimal in-test document for the edit tests: scalar fields with
//! storage bounds and caps, and one ordered collection keyed by `u32`.
//! Each test crate declares `mod common;`: Cargo does not build this file as
//! a test target.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::collections::BTreeMap;

use neoffse_core::{CapPolicy, Edit, EditError, FieldId, Key, RowId, Value, ValueKind};

/// The only collection of the model.
const INVENTORY: &str = "inventory";
/// Row keys of `inventory` that name an entry of the (imaginary) ID table.
const KEYS: std::ops::RangeInclusive<u32> = 1..=200;
/// Storage bounds of a row's `count`.
const COUNT_MIN: i64 = 1;
const COUNT_MAX: i64 = 99;

/// One scalar field: its value and the rules an edit meets.
#[derive(Debug, Clone)]
struct Slot {
    value: Value,
    bounds: (i64, i64),
    cap: Option<i64>,
    read_only: bool,
}

impl Slot {
    fn int(value: i64, bounds: (i64, i64), cap: Option<i64>) -> Self {
        Self {
            value: Value::Int(value),
            bounds,
            cap,
            read_only: false,
        }
    }

    fn other(value: Value) -> Self {
        Self {
            value,
            bounds: (0, 0),
            cap: None,
            read_only: false,
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

/// The storage bound first, then the cap under `Enforce`.
fn checked_int(field: &FieldId, value: &Value, bounds: (i64, i64), cap: Option<i64>, policy: CapPolicy) -> Result<i64, EditError> {
    let Value::Int(number) = value else {
        return Err(EditError::WrongType {
            field: field.clone(),
            expected: ValueKind::Int,
            found: value.kind(),
        });
    };
    let (min, max) = bounds;
    if *number < min || *number > max {
        return Err(EditError::OutOfRange {
            field: field.clone(),
            value: *number,
            min,
            max,
        });
    }
    if let Some(cap) = cap
        && policy == CapPolicy::Enforce
        && *number > cap
    {
        return Err(EditError::AboveCap {
            field: field.clone(),
            value: *number,
            cap,
        });
    }
    Ok(*number)
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
        locked.read_only = true;
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
            let count = checked_int(field, value, (COUNT_MIN, COUNT_MAX), None, policy)?;
            let Some(slot) = self.rows.get_mut(index) else {
                return Err(EditError::UnknownField { field: field.clone() });
            };
            let old = std::mem::replace(&mut slot.1, count);
            return Ok(Edit::set(field.clone(), Value::Int(old)));
        }
        let Some(slot) = self.scalars.get_mut(field) else {
            return Err(EditError::UnknownField { field: field.clone() });
        };
        if slot.read_only {
            return Err(EditError::ReadOnly { field: field.clone() });
        }
        if slot.value.kind() != value.kind() {
            return Err(EditError::WrongType {
                field: field.clone(),
                expected: slot.value.kind(),
                found: value.kind(),
            });
        }
        if matches!(value, Value::Int(_)) {
            checked_int(field, value, slot.bounds, slot.cap, policy)?;
        }
        let old = std::mem::replace(&mut slot.value, value.clone());
        Ok(Edit::set(field.clone(), old))
    }

    fn insert(&mut self, id: &RowId, position: Option<usize>, values: &[(&'static str, Value)], policy: CapPolicy) -> Result<Edit, EditError> {
        let collection = id.collection();
        if collection != FieldId::new(INVENTORY) {
            return Err(EditError::NotACollection { field: collection });
        }
        if !KEYS.contains(&id.key()) {
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
        let mut count = None;
        for (name, value) in values {
            if *name != "count" || count.is_some() {
                return Err(EditError::UnknownField { field: id.field(name) });
            }
            count = Some(checked_int(&id.field(name), value, (COUNT_MIN, COUNT_MAX), None, policy)?);
        }
        self.rows.insert(position, (id.key(), count.unwrap_or(1)));
        Ok(Edit::remove(id.clone()))
    }

    fn remove(&mut self, id: &RowId) -> Result<Edit, EditError> {
        let collection = id.collection();
        if collection != FieldId::new(INVENTORY) {
            return Err(EditError::NotACollection { field: collection });
        }
        let Some(index) = self.index_of(id.key()) else {
            return Err(EditError::MissingRow { row: id.id().clone() });
        };
        let (_, count) = self.rows.remove(index);
        Ok(Edit::insert(id.clone(), Some(index), vec![("count", Value::Int(count))]))
    }
}
