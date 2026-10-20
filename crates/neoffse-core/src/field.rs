//! Field and row identity, and the values an edit writes.

use std::fmt;

/// One step of a field path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Key {
    /// A name chosen by the format, ASCII snake case: `character`, `max_hp`.
    Name(&'static str),
    /// A number the save assigns: a character `id`, a `contentId`, a slot.
    Id(u32),
}

/// The identity of one editable field: a section name, then names and ids.
///
/// A path reads like `gil`, `character.1.max_hp` or `inventory.7.count`. The
/// names here are illustrative; the format that produced an id owns the real
/// ones and resolves the id to a place in its document.
///
/// The builders consume `self`: clone the parent to derive two children. Two
/// ids are equal when their key lists are equal, so `Name("1")` differs from
/// `Id(1)`. `Display` joins the keys with `.` for logs and error messages;
/// do not parse it.
///
/// Paths are keyed data and not an enum because each game gets an edit page
/// that mirrors the game's own menu: the format declares its paths and the
/// app lays them out.
///
/// ```
/// use neoffse_core::FieldId;
///
/// let max_hp = FieldId::new("character").with_id(1).with_name("max_hp");
/// assert_eq!(max_hp.to_string(), "character.1.max_hp");
/// assert_eq!(max_hp.section(), "character");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FieldId {
    /// Never empty; the first key is a `Key::Name`.
    keys: Vec<Key>,
}

impl FieldId {
    /// A path made of its section name alone.
    #[must_use]
    pub fn new(section: &'static str) -> Self {
        Self {
            keys: vec![Key::Name(section)],
        }
    }

    /// The path extended with a name.
    #[must_use]
    pub fn with_name(mut self, name: &'static str) -> Self {
        self.keys.push(Key::Name(name));
        self
    }

    /// The path extended with a numeric id.
    #[must_use]
    pub fn with_id(mut self, id: u32) -> Self {
        self.keys.push(Key::Id(id));
        self
    }

    /// The name of the first key. Returns `""` for a state that construction
    /// rules out.
    #[must_use]
    pub fn section(&self) -> &'static str {
        match self.keys.first() {
            Some(Key::Name(name)) => name,
            _ => "",
        }
    }

    /// Every key, section first.
    #[must_use]
    pub fn keys(&self) -> &[Key] {
        &self.keys
    }

    /// The path without its last key; `None` for a section alone.
    #[must_use]
    pub fn parent(&self) -> Option<FieldId> {
        let (_, rest) = self.keys.split_last()?;
        if rest.is_empty() {
            return None;
        }
        Some(FieldId { keys: rest.to_vec() })
    }

    /// Whether `prefix` is a prefix of this path, key by key. A path is a
    /// prefix of itself.
    #[must_use]
    pub fn starts_with(&self, prefix: &FieldId) -> bool {
        self.keys.starts_with(&prefix.keys)
    }
}

impl fmt::Display for FieldId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, key) in self.keys.iter().enumerate() {
            if index > 0 {
                f.write_str(".")?;
            }
            match key {
                Key::Name(name) => f.write_str(name)?,
                Key::Id(id) => write!(f, "{id}")?,
            }
        }
        Ok(())
    }
}

/// The identity of one row of a collection: the collection's path plus a
/// numeric key. `inventory` and 7 give the row `inventory.7`; its fields are
/// `inventory.7.count`, `inventory.7.name`, and so on.
///
/// The key is a `u32`: the `contentId` of an inventory entry, the `abilityId`
/// of a spell. A collection keeps the game's own order, and the position of a
/// row is not part of its identity.
///
/// ```
/// use neoffse_core::{FieldId, RowId};
///
/// let row = RowId::new(FieldId::new("inventory"), 7);
/// assert_eq!(row.field("count").to_string(), "inventory.7.count");
/// assert_eq!(row.key(), 7);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RowId {
    /// The collection path plus the key; the last key is a `Key::Id`.
    id: FieldId,
}

impl RowId {
    /// The row `key` of `collection`.
    #[must_use]
    pub fn new(collection: FieldId, key: u32) -> Self {
        Self { id: collection.with_id(key) }
    }

    /// The path of the collection, without the key. Returns
    /// `FieldId::new("")` for a state that construction rules out.
    #[must_use]
    pub fn collection(&self) -> FieldId {
        self.id.parent().unwrap_or_else(|| FieldId::new(""))
    }

    /// The numeric key of the row. Returns `0` for a state that construction
    /// rules out.
    #[must_use]
    pub fn key(&self) -> u32 {
        match self.id.keys().last() {
            Some(Key::Id(key)) => *key,
            _ => 0,
        }
    }

    /// The collection path plus the key: the prefix of the row's field ids.
    #[must_use]
    pub fn id(&self) -> &FieldId {
        &self.id
    }

    /// The id of the field `name` under this row.
    #[must_use]
    pub fn field(&self, name: &'static str) -> FieldId {
        self.id.clone().with_name(name)
    }
}

impl fmt::Display for RowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.id.fmt(f)
    }
}

/// The value an edit writes or a field holds.
///
/// An integer holds an `i64`: the saves' values fit (`docs/formats/pixel-remaster.md#numbers`
/// lists the `u32` flags and the `i64` ticks). A reference into an ID table
/// is an `Int`, and the field's metadata says which table. Floats are absent
/// on purpose, since `f64` has no `Eq`; a later version may add a variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Value {
    /// A whole number.
    Int(i64),
    /// A flag.
    Bool(bool),
    /// A string.
    Text(String),
}

impl Value {
    /// The type of the value.
    #[must_use]
    pub fn kind(&self) -> ValueKind {
        match self {
            Value::Int(_) => ValueKind::Int,
            Value::Bool(_) => ValueKind::Bool,
            Value::Text(_) => ValueKind::Text,
        }
    }
}

/// The type of a [`Value`], for error messages and metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValueKind {
    /// A whole number.
    Int,
    /// A flag.
    Bool,
    /// A string.
    Text,
}

impl fmt::Display for ValueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ValueKind::Int => "integer",
            ValueKind::Bool => "boolean",
            ValueKind::Text => "text",
        })
    }
}
