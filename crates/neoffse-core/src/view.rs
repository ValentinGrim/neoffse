//! What the app reads from an open save: the summary for the Home screen and
//! the view for the Editor.
//!
//! `Debug` output of these types holds save content (character names, the
//! location). A caller must not log it (`docs/architecture.md#errors`,
//! personal data).

use std::time::Duration;

use crate::collection::CollectionMeta;
use crate::field::{FieldId, Value};
use crate::game::Game;
use crate::kinds::Capability;
use crate::meta::FieldMeta;

/// A save slot as the player sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum SaveSlot {
    /// A manual slot, numbered from 1.
    Numbered(u32),
    /// The automatic save.
    Auto,
    /// The quick save.
    Quick,
}

/// What the Home screen shows of a save.
///
/// `Debug` output holds save content (character names, the location): a
/// caller must not log it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SaveSummary {
    /// The game the save belongs to.
    pub game: Game,
    /// The slot the file occupies.
    pub slot: Option<SaveSlot>,
    /// The time played.
    pub play_time: Option<Duration>,
    /// The name of the place the party stands in.
    pub location: Option<String>,
    /// The names of the party, in the order the save lists them.
    pub party: Vec<String>,
}

impl SaveSummary {
    /// A summary of `game` with every other field empty.
    #[must_use]
    pub fn new(game: Game) -> Self {
        Self {
            game,
            slot: None,
            play_time: None,
            location: None,
            party: Vec::new(),
        }
    }
}

/// The editable content of a save, one section per capability.
///
/// The view is a snapshot: a later `apply` does not change a view already
/// returned, and the view model asks the document for a fresh one. `Debug`
/// output holds save content (character names, values): a caller must not
/// log it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SaveView {
    /// The sections, in display order.
    pub sections: Vec<Section>,
}

impl SaveView {
    /// A view of `sections`.
    #[must_use]
    pub fn new(sections: Vec<Section>) -> Self {
        Self { sections }
    }

    /// The first section of `capability`.
    #[must_use]
    pub fn section(&self, capability: Capability) -> Option<&Section> {
        self.sections.iter().find(|section| section.capability == capability)
    }

    /// The entity and the field view whose `FieldView::id` equals `id`.
    /// Matches ids, never labels. The first match in section, entity and
    /// field order wins.
    #[must_use]
    pub fn find(&self, id: &FieldId) -> Option<(&Entity, &FieldView)> {
        self.sections
            .iter()
            .flat_map(|section| &section.entities)
            .find_map(|entity| entity.fields.iter().find(|field| field.id == *id).map(|field| (entity, field)))
    }

    /// The metadata of the collection whose `CollectionMeta::id` equals `id`.
    /// The first match in section and collection order wins.
    #[must_use]
    pub fn collection(&self, id: &FieldId) -> Option<&CollectionMeta> {
        self.sections.iter().flat_map(|section| &section.collections).find(|meta| meta.id == *id)
    }
}

/// The part of the view under one capability.
///
/// A section lists a collection's rows as entities in the document's order,
/// and the matching [`CollectionMeta`] sits in
/// [`collections`](Section::collections), so the UI shows the rows and the
/// add and remove controls the flags allow. Gil belongs to
/// `Capability::Inventory`.
///
/// `Debug` prints save content: never log it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Section {
    /// The capability the section serves.
    pub capability: Capability,
    /// The things that own fields, in display order.
    pub entities: Vec<Entity>,
    /// The collections this section edits; empty for a section with none.
    pub collections: Vec<CollectionMeta>,
}

impl Section {
    /// A section with no collection metadata.
    #[must_use]
    pub fn new(capability: Capability, entities: Vec<Entity>) -> Self {
        Self {
            capability,
            entities,
            collections: Vec::new(),
        }
    }
}

/// A character, an item row or any other thing that owns fields.
///
/// `id` is the prefix every one of its fields' ids starts with
/// (`FieldId::starts_with`); for a row of a collection it is `RowId::id()`.
/// The view enforces neither rule.
///
/// `Debug` prints save content: never log it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Entity {
    /// The prefix of the entity's field ids.
    pub id: FieldId,
    /// The text that names the entity.
    pub label: String,
    /// The fields, in display order.
    pub fields: Vec<FieldView>,
}

impl Entity {
    /// An entity from its parts.
    #[must_use]
    pub fn new(id: FieldId, label: impl Into<String>, fields: Vec<FieldView>) -> Self {
        Self {
            id,
            label: label.into(),
            fields,
        }
    }
}

/// One field with its metadata and its current value.
///
/// `Debug` prints save content: never log it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct FieldView {
    /// The path an `Edit::Set` addresses.
    pub id: FieldId,
    /// What the editor needs to show and check the field.
    pub meta: FieldMeta,
    /// The value the document holds now.
    pub value: Value,
}

impl FieldView {
    /// A field view from its parts.
    #[must_use]
    pub fn new(id: FieldId, meta: FieldMeta, value: Value) -> Self {
        Self { id, meta, value }
    }
}
