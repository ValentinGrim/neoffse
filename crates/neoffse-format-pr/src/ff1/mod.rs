//! The FF I Pixel Remaster slot as the domain model of `neoffse-core`.
//!
//! [`Ff1Model`] maps the lossless [`Document`] of a slot onto the field
//! vocabulary of [`names`]: characters (name, job, level, experience, HP,
//! stats, spell charges, five equipment slots), the two item lists and gil
//! (`docs/formats/pixel-remaster.md#characters`,
//! `docs/formats/pixel-remaster.md#equipment-slots`,
//! `docs/formats/pixel-remaster.md#inventory`). A read finds a character by
//! its `id`, an item by its `contentId` and a dictionary entry by its key,
//! never by position. The model keeps no state beside the document: every
//! key it does not map stays as read.

mod apply;
mod binding;
mod locate;
mod meta;
pub mod names;

use neoffse_core::{Capability, CollectionMeta, EditError, FieldId, FieldMeta, Game, Value};
use neoffse_data::GameTables;

use crate::document::{Document, DocumentError, Scalar, Segment};
use binding::{Binding, Collection};
use locate::{format_error, locate, read_int, read_text, row_keys};

/// The game of this module.
pub const GAME: Game = Game::Ff1Pr;

/// The capabilities of an FF I slot: the party and the inventory (gil sits
/// under the inventory).
pub const CAPABILITIES: [Capability; 2] = [Capability::Party, Capability::Inventory];

/// An FF I slot document with its ID tables.
///
/// `Debug` prints save content (names): callers must not log it.
#[derive(Debug, Clone)]
pub struct Ff1Model {
    document: Document,
    tables: &'static GameTables,
}

impl Ff1Model {
    /// Wraps `document`. Nothing is validated: a document of another shape
    /// fails when a field reads it.
    #[must_use]
    pub fn new(document: Document, tables: &'static GameTables) -> Self {
        Self { document, tables }
    }

    /// The document, for `to_json`.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The document, consumed.
    #[must_use]
    pub fn into_document(self) -> Document {
        self.document
    }

    /// Reads any path of the document without writing. It reaches the keys
    /// the model does not map, such as `id` and `userData.playTime`.
    ///
    /// # Errors
    ///
    /// The `DocumentError` of [`Document::get`].
    pub fn read(&mut self, path: &[Segment<'_>]) -> Result<Scalar, DocumentError> {
        self.document.get(path)
    }

    /// The row keys of `collection` in document order: the `id` of each
    /// character, or the `contentId` of each row of `inventory` or
    /// `key_items`.
    ///
    /// # Errors
    ///
    /// `NotACollection` when `collection` is none of the three, `Format` when
    /// the document does not hold the list or a key.
    pub fn rows(&mut self, collection: &FieldId) -> Result<Vec<u32>, EditError> {
        let kind = Collection::of(collection).ok_or_else(|| EditError::NotACollection { field: collection.clone() })?;
        row_keys(&mut self.document, kind).map_err(|source| format_error(collection, source))
    }

    /// The value of `field`.
    ///
    /// # Errors
    ///
    /// `UnknownField` for an undeclared path or a dictionary without the key,
    /// `MissingRow` for an absent character or item, `Format` when the
    /// document holds another shape: a float or an oversized integer where
    /// an integer belongs, a non-string name.
    pub fn get(&mut self, field: &FieldId) -> Result<Value, EditError> {
        let binding = Binding::resolve(field)?;
        let path = locate(&mut self.document, &binding, field)?;
        let read = if binding.is_text() {
            read_text(&mut self.document, &path).map(Value::Text)
        } else {
            read_int(&mut self.document, &path).map(Value::Int)
        };
        read.map_err(|source| format_error(field, source))
    }

    /// The metadata of `field`, or `None` for an undeclared path. It reads
    /// no document: a path that names an absent row still has a meta.
    #[must_use]
    pub fn field_meta(&self, field: &FieldId) -> Option<FieldMeta> {
        let binding = Binding::resolve(field).ok()?;
        Some(meta::field_meta(&binding, self.tables))
    }

    /// The metadata of `character`, `inventory` or `key_items`, `None` for
    /// any other path.
    #[must_use]
    pub fn collection_meta(&self, collection: &FieldId) -> Option<CollectionMeta> {
        Some(meta::collection_meta(Collection::of(collection)?, self.tables))
    }
}
