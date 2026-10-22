//! The net difference between two views of one save.
//!
//! The app shows it before a save: the pending changes bar counts the
//! edits, and the review lists what differs from the clean state
//! (`docs/architecture.md#ui-architecture`). The user reviews this summary
//! before the write (`docs/architecture.md#save-safety`, step 2).
//!
//! [`diff`] is pure and format-agnostic: it reads two [`SaveView`]s and
//! names no game and no field. A value set and then set back gives no
//! [`Change`], and a row that only moved gives none either.
//! [`Session::changes`](crate::Session::changes) builds the two views.

use std::collections::{HashMap, HashSet};

use neoffse_core::{Capability, Entity, FieldId, FieldMeta, FieldView, SaveView, Section, Value};
use neoffse_data::{GameTables, Language};

/// One difference between the clean state and the current state of a session.
///
/// `Debug` prints save content: never log it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Change {
    /// The section the entity belongs to.
    pub capability: Capability,
    /// The entity: a character, a row of a collection, the gil.
    pub entity: FieldId,
    /// The entity's label in the current state; in the clean state for a
    /// removed row.
    pub entity_label: String,
    /// What differs.
    pub kind: ChangeKind,
}

impl Change {
    /// A change of `kind` on the entity `entity`, shown as `entity_label`.
    #[must_use]
    pub fn new(capability: Capability, entity: FieldId, entity_label: impl Into<String>, kind: ChangeKind) -> Self {
        Self {
            capability,
            entity,
            entity_label: entity_label.into(),
            kind,
        }
    }
}

/// What differs on one entity.
///
/// `Debug` prints save content: never log it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChangeKind {
    /// A field whose value differs. `before` and `after` are display texts:
    /// empty when the field exists on the other side only.
    Field {
        /// The field.
        field: FieldId,
        /// The field's label.
        label: String,
        /// The clean value as text.
        before: String,
        /// The current value as text.
        after: String,
    },
    /// The entity exists only in the current state.
    RowAdded,
    /// The entity exists only in the clean state.
    RowRemoved,
}

/// The net difference of two views, in the order of `after`.
///
/// Sections pair by capability, entities by id, fields by id. Fields whose
/// metadata says read-only are left out: they follow from other fields, and
/// those fields already show. A removed entity comes after the other
/// changes of its section, a section only in `before` last.
///
/// An `Int` shows its name from the field's metadata when it has one. Else
/// it shows the name of the first table of the metadata that holds the id,
/// in `language`. Else it shows the number.
///
/// ```
/// use neoffse::changes::{Change, ChangeKind, diff};
/// use neoffse::core::{Capability, Entity, FieldId, FieldMeta, FieldView, SaveView, Section, Shape, Value};
/// use neoffse::data::Language;
///
/// let gil = FieldId::new("gil");
/// let view = |amount| {
///     let field = FieldView::new(gil.clone(), FieldMeta::new("Gil", Shape::Int { min: 0, max: 999 }), Value::Int(amount));
///     let entity = Entity::new(gil.clone(), "Gil", vec![field]);
///     SaveView::new(vec![Section::new(Capability::Inventory, vec![entity])])
/// };
///
/// let changes = diff(&view(10), &view(12), None, Language::En);
/// let kind = ChangeKind::Field { field: gil.clone(), label: "Gil".into(), before: "10".into(), after: "12".into() };
/// assert_eq!(changes, [Change::new(Capability::Inventory, gil, "Gil", kind)]);
/// ```
#[must_use]
pub fn diff(before: &SaveView, after: &SaveView, tables: Option<&GameTables>, language: Language) -> Vec<Change> {
    let texts = Texts { tables, language };
    let mut changes = Vec::new();
    for section in &after.sections {
        match before.section(section.capability) {
            Some(old) => diff_section(old, section, &texts, &mut changes),
            None => {
                for entity in &section.entities {
                    changes.push(whole(section.capability, entity, ChangeKind::RowAdded));
                }
            }
        }
    }
    for section in &before.sections {
        if after.section(section.capability).is_none() {
            for entity in &section.entities {
                changes.push(whole(section.capability, entity, ChangeKind::RowRemoved));
            }
        }
    }
    changes
}

/// The tables and the language that turn a value into a display text.
struct Texts<'a> {
    tables: Option<&'a GameTables>,
    language: Language,
}

impl Texts<'_> {
    fn show(&self, value: &Value, meta: &FieldMeta) -> String {
        match value {
            Value::Text(text) => text.clone(),
            Value::Bool(true) => "Yes".to_owned(),
            Value::Bool(false) => "No".to_owned(),
            Value::Int(number) => self.show_int(*number, meta),
            // A kind added to the model later: show it rather than hide it.
            other => format!("{other:?}"),
        }
    }

    fn show_int(&self, number: i64, meta: &FieldMeta) -> String {
        if let Some(name) = meta.name_of(number) {
            return name.to_owned();
        }
        if let (Some(tables), Ok(id)) = (self.tables, u32::try_from(number)) {
            for kind in &meta.tables {
                if let Some(entry) = tables.entry(*kind, id) {
                    let name = entry.name(self.language);
                    if !name.is_empty() {
                        return name.to_owned();
                    }
                }
            }
        }
        number.to_string()
    }
}

fn whole(capability: Capability, entity: &Entity, kind: ChangeKind) -> Change {
    Change::new(capability, entity.id.clone(), entity.label.clone(), kind)
}

/// Pairs the entities of two sections that share a capability.
fn diff_section(before: &Section, after: &Section, texts: &Texts<'_>, changes: &mut Vec<Change>) {
    let mut old: HashMap<&FieldId, &Entity> = HashMap::new();
    for entity in &before.entities {
        old.entry(&entity.id).or_insert(entity);
    }
    for entity in &after.entities {
        match old.get(&entity.id) {
            Some(counterpart) => diff_entity(before.capability, counterpart, entity, texts, changes),
            None => changes.push(whole(after.capability, entity, ChangeKind::RowAdded)),
        }
    }
    let kept: HashSet<&FieldId> = after.entities.iter().map(|entity| &entity.id).collect();
    for entity in before.entities.iter().filter(|entity| !kept.contains(&entity.id)) {
        changes.push(whole(before.capability, entity, ChangeKind::RowRemoved));
    }
}

/// The field changes of one entity present on both sides.
fn diff_entity(capability: Capability, before: &Entity, after: &Entity, texts: &Texts<'_>, changes: &mut Vec<Change>) {
    let mut old: HashMap<&FieldId, &FieldView> = HashMap::new();
    for field in &before.fields {
        old.entry(&field.id).or_insert(field);
    }
    for field in after.fields.iter().filter(|field| !field.meta.read_only) {
        let before_text = match old.get(&field.id) {
            Some(counterpart) if counterpart.value == field.value => continue,
            Some(counterpart) => texts.show(&counterpart.value, &field.meta),
            None => String::new(),
        };
        let kind = ChangeKind::Field {
            field: field.id.clone(),
            label: field.meta.label.clone(),
            before: before_text,
            after: texts.show(&field.value, &field.meta),
        };
        changes.push(Change::new(capability, after.id.clone(), after.label.clone(), kind));
    }
    let present: HashSet<&FieldId> = after.fields.iter().map(|field| &field.id).collect();
    for field in before.fields.iter().filter(|field| !field.meta.read_only && !present.contains(&field.id)) {
        let kind = ChangeKind::Field {
            field: field.id.clone(),
            label: field.meta.label.clone(),
            before: texts.show(&field.value, &field.meta),
            after: String::new(),
        };
        changes.push(Change::new(capability, after.id.clone(), after.label.clone(), kind));
    }
}
