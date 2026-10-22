//! The FF I Pixel Remaster slot as the domain model of `neoffse-core`.
//!
//! [`Ff1Model`] maps the lossless [`Document`] of a slot onto the field
//! vocabulary of [`names`]: characters (name, job, level, experience, HP,
//! stats, spell charges, five equipment slots, spell slots, and the two
//! computed read-only fields `max_hp` and `next_level`), the two item
//! lists and gil (`docs/formats/pixel-remaster.md#characters`,
//! `docs/formats/pixel-remaster.md#equipment-slots`,
//! `docs/formats/pixel-remaster.md#inventory`). A read finds a character by
//! its `id`, an item by its `contentId` and a dictionary entry by its key,
//! never by position. The model keeps no state beside the document: every
//! key it does not map stays as read.
//!
//! The model edits through two interfaces over the same `Edit`:
//!
//! - [`Ff1Model::apply`] is the raw interface. It checks the storage bounds
//!   of the field and, under `CapPolicy::Enforce`, the game caps and the ID
//!   tables, and it holds no game rule: a bare equipment id, a count below the
//!   number of wearers and a placeholder row all pass when the value is in
//!   bounds. `CapPolicy::StorageOnly` is the expert mode of an app.
//! - [`Ff1Model::apply_game`] edits the way the game would. It enforces caps
//!   and tables, expands an equipment `Set` into the slot, the inventory row
//!   and the slot count, sets Level and Experience together, refuses the edits the game never makes, and returns a
//!   `GameChange`. [`Ff1Model::row_locked`] and [`Ff1Model::field_locked`] name
//!   the rows and fields it refuses whatever the value. A spell slot is read
//!   only in both interfaces.
//!
//! [`Ff1Model::summary`] and [`Ff1Model::view`] read the slot for the Home
//! screen and the editor: the first gives the slot, play time, place and
//! party, the second a snapshot of every character, item row and gil and the
//! spell slots of every character. Both leave the document untouched.
//!
//! [`Ff1Format`] implements `neoffse_core::SaveFormat` for the game: it
//! detects a slot file by its name and opens it as an [`Ff1Document`], which
//! implements `neoffse_core::SaveDocument` over the model, the container
//! facts of the opened file and its name. [`OpenError`] says why a file is no
//! slot of this game.
//!
//! Open a slot, change the gil the way the game would, and encode the file:
//!
//! ```
//! use neoffse_core::{Edit, FieldId, SaveDocument, SaveFiles, Value};
//! use neoffse_format_pr::container::{Container, encode};
//! use neoffse_format_pr::ff1::{Ff1Format, names};
//!
//! // A synthetic slot of the repository, wrapped in the game's container.
//! let json = neoffse_testkit::read_fixture("pr/ff1-slot.json")?;
//! let mut files = SaveFiles::new();
//! files.push("slot.sav", encode(std::str::from_utf8(&json)?, &Container::game())?)?;
//!
//! let mut document = Ff1Format::new().open_document(&files)?;
//! let gil = FieldId::new(names::GIL);
//! let change = document.apply_game(&Edit::set(gil, Value::Int(500)))?;
//! assert_eq!(change.forward.len(), 1);
//!
//! let saved = document.encode()?;
//! document.verify(&saved)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod apply;
mod binding;
mod derived;
mod format;
mod game;
mod level;
mod lists;
mod locate;
mod meta;
pub mod names;
mod save;
mod summary;
mod view;

pub use neoffse_core::GameChange;
use neoffse_core::{Capability, CollectionMeta, EditError, FieldId, FieldMeta, Game, Value};
use neoffse_data::GameTables;

use crate::document::{Document, DocumentError, Scalar, Segment};
pub use crate::open_error::OpenError;
use binding::{Binding, Collection};
pub use format::Ff1Format;
use locate::{format_error, locate, read_ability, read_int, read_text, row_keys};
pub use save::Ff1Document;

/// The game of this module.
pub const GAME: Game = Game::Ff1Pr;

/// The capabilities of an FF I slot: the party, the inventory (gil sits under
/// the inventory) and the magic (the spell slots, read only).
pub const CAPABILITIES: [Capability; 3] = [Capability::Party, Capability::Inventory, Capability::Magic];

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
    ///
    /// A lookup by `id` or `contentId` takes the first row with that key.
    /// [`Ff1Format::open_document`](Ff1Format::open_document) refuses a
    /// document that repeats a key; a model built here accepts it.
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

    /// The value of `field`. `max_hp` and `next_level` are computed from the
    /// document and the tables: the base HP of the character's status row plus
    /// the stored gain, held to the `MAX_HP` cap, and the experience still
    /// needed for the next level, read from `currentExp` alone.
    ///
    /// # Errors
    ///
    /// `UnknownField` for an undeclared path, a dictionary without the key, a
    /// computed field whose key or table row is missing (and `next_level` at
    /// level 99 or with a negative experience),
    /// `MissingRow` for an absent character or item, `Format` when the
    /// document holds another shape: a float or an oversized integer where
    /// an integer belongs, a non-string name.
    pub fn get(&mut self, field: &FieldId) -> Result<Value, EditError> {
        let binding = Binding::resolve(field)?;
        if let Binding::Character { id, part } = binding
            && binding.is_derived()
        {
            return derived::value(&mut self.document, self.tables, id, part, field);
        }
        let path = locate(&mut self.document, &binding, field)?;
        let read = if binding.is_text() {
            read_text(&mut self.document, &path).map(Value::Text)
        } else if binding.is_spell() {
            read_ability(&mut self.document, &path).map(|id| Value::Int(i64::from(id)))
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
