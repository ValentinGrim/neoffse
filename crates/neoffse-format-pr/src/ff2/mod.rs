//! The FF II Pixel Remaster slot as the domain model of `neoffse-core`.
//!
//! [`Ff2Model`] maps the lossless [`Document`] of a slot onto the field
//! vocabulary of [`names`]: characters (name, person, HP, MP, the gains of
//! maximum HP, maximum MP and the six stats, the ten skill values, five
//! equipment slots, and the two computed read-only fields `max_hp` and
//! `max_mp`), the two item lists and gil
//! (`docs/formats/pixel-remaster.md#characters`,
//! `docs/formats/pixel-remaster.md#skill-levels-in-ff-ii`,
//! `docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`,
//! `docs/formats/pixel-remaster.md#inventory`) and the keywords
//! (`docs/formats/pixel-remaster.md#keywords-in-ff-ii`). A read finds a character by
//! its `id`, an item by its `contentId` and a dictionary entry by its key,
//! never by position. The model keeps no state beside the document: every
//! key it does not map stays as read.
//!
//! FF II has no level, experience or spell charge field, so those paths of
//! the FF I vocabulary are unknown here
//! (`docs/formats/pixel-remaster.md#character-growth-in-ff-ii`). A spell has
//! one field, `character.<id>.spell.<chain>`, its level: the model reads it
//! from the row of the spell's `abilityList` entry
//! (`docs/formats/pixel-remaster.md#spells-in-ff-ii`) and a raw `Set` of it is
//! `ReadOnly`. Each stored place of a spell is a field of its own, addressed
//! by its position in its list (`character.<id>.ability.<i>.<part>`,
//! `dictionary.<j>.<part>`, `addition_order.<k>`, `sort_order.<k>`): the raw
//! interface edits it with an exact inverse, and the game interface refuses it
//! as a locked field.
//!
//! The model edits through two interfaces over the same `Edit`:
//!
//! - [`Ff2Model::apply`] is the raw interface. It checks the storage bounds
//!   of the field and, under `CapPolicy::Enforce`, the game caps and the ID
//!   tables, and it holds no game rule: a bare equipment id, a count below
//!   the number of wearers, a gain past `cap - base` and a hand set to the
//!   shield placeholder all pass when the value is in bounds.
//!   `CapPolicy::StorageOnly` is the expert mode of an app.
//! - [`Ff2Model::apply_game`] edits the way the game would. It enforces caps
//!   and tables, expands an equipment `Set` into the slot, the inventory row
//!   and the slot count, writes a spell's level to the five places the game
//!   changes together, refuses the edits the game never makes, and returns
//!   a `GameChange`. [`Ff2Model::row_locked`] and [`Ff2Model::field_locked`]
//!   name the rows and fields it refuses whatever the value.
//!
//! [`Ff2Model::summary`] and [`Ff2Model::view`] read the slot for the Home
//! screen and the editor: the first gives the slot, play time, place and
//! party, the second a snapshot of every character, item row, gil and spell
//! level. Both leave the document untouched.
//!
//! [`Ff2Format`] implements `neoffse_core::SaveFormat` for the game: it
//! detects a slot file by its name and opens it as an [`Ff2Document`], which
//! implements `neoffse_core::SaveDocument` over the model, the container
//! facts of the opened file and its name. Both are aliases of the generic
//! [`SlotFormat`] and [`SlotDocument`] of [`crate::shared`], instantiated
//! with [`Ff2Model`]; the impl of `SlotModel` for the model lives in this
//! module.

mod apply;
mod binding;
mod derived;
mod game;
mod keywords;
mod locate;
mod meta;
pub mod names;
mod places;
mod slot;
mod spell;
mod spell_plan;
mod summary;
mod view;

use neoffse_core::{Capability, CollectionMeta, EditError, FieldId, FieldMeta, Game, Value};
use neoffse_data::GameTables;

use crate::document::{Document, DocumentError, Scalar, Segment};
use crate::shared::meta::collection_meta;
use crate::shared::paths::Collection;
use crate::shared::read::{format_error, read_int, read_text, row_keys};
use crate::shared::{SlotDocument, SlotFormat};
use binding::{Binding, Part};
use locate::locate;

/// The game of this module.
pub const GAME: Game = Game::Ff2Pr;

/// The capabilities of an FF II slot: the party, the inventory (gil sits
/// under the inventory) and the magic (the level of each spell).
pub const CAPABILITIES: [Capability; 3] = [Capability::Party, Capability::Inventory, Capability::Magic];

/// The FF II Pixel Remaster format of `neoffse-core`: detection by slot name
/// and the typed open.
///
/// It holds the ID tables and the language of the view labels, fixed for
/// every document it opens: a language switch builds a new format. It is an
/// alias of the generic [`SlotFormat`] of the shared layer.
pub type Ff2Format = SlotFormat<Ff2Model>;

/// An FF II slot opened by [`Ff2Format`].
///
/// It keeps the BOM and the exact trailer of the opened file and the file
/// name, so `encode` writes the file back as it came. Its `Debug` is written
/// by hand and prints the file name only, and it has no `Clone`.
pub type Ff2Document = SlotDocument<Ff2Model>;

/// An FF II slot document with its ID tables.
///
/// `Debug` prints save content (names): callers must not log it.
#[derive(Debug, Clone)]
pub struct Ff2Model {
    document: Document,
    tables: &'static GameTables,
}

impl Ff2Model {
    /// Wraps `document`. Nothing is validated: a document of another shape
    /// fails when a field reads it.
    ///
    /// A lookup by `id` or `contentId` takes the first row with that key.
    /// The shared open pipeline refuses a document that repeats a key; a
    /// model built here accepts it.
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
    /// character, the `contentId` of each row of `inventory` or `key_items`,
    /// or each integer of `keywords`.
    ///
    /// # Errors
    ///
    /// `NotACollection` when `collection` is none of the four, `Format` when
    /// the document does not hold the list or a key.
    pub fn rows(&mut self, collection: &FieldId) -> Result<Vec<u32>, EditError> {
        if keywords::owns(collection) {
            return keywords::rows(&mut self.document).map_err(|source| format_error(collection, source));
        }
        let kind = Collection::of(collection).ok_or_else(|| EditError::NotACollection { field: collection.clone() })?;
        row_keys(&mut self.document, kind).map_err(|source| format_error(collection, source))
    }

    /// The value of `field`. `max_hp` and `max_mp` are computed from the
    /// document and the tables: the base of the character's status row plus
    /// the stored gain, held to the `MAX_HP` or `MAX_MP` cap. A spell level is
    /// the level of the row of the first `abilityList` entry in its chain.
    ///
    /// # Errors
    ///
    /// `UnknownField` for an undeclared path (FF I's `level`, `exp`,
    /// `charges` and `spell.<level>.<slot>` among them), a dictionary without
    /// the key, a spell chain the character owns no entry of, a spell place past
    /// the end of its list, or a computed
    /// field whose key or table row is missing, `MissingRow` for an
    /// absent character or item, `Format` when the document holds another
    /// shape: a float or an oversized integer where an integer belongs, a
    /// non-string name.
    pub fn get(&mut self, field: &FieldId) -> Result<Value, EditError> {
        get_in(&mut self.document, self.tables, field)
    }

    /// The metadata of `field`, or `None` for an undeclared path. It reads
    /// no document: a path that names an absent row still has a meta.
    #[must_use]
    pub fn field_meta(&self, field: &FieldId) -> Option<FieldMeta> {
        let binding = Binding::resolve(field).ok()?;
        Some(meta::field_meta(&binding, self.tables))
    }

    /// The metadata of `character`, `inventory`, `key_items` or `keywords`,
    /// `None` for any other path.
    #[must_use]
    pub fn collection_meta(&self, collection: &FieldId) -> Option<CollectionMeta> {
        if keywords::owns(collection) {
            return Some(keywords::meta());
        }
        Some(collection_meta(Collection::of(collection)?, self.tables))
    }
}

/// The body of [`Ff2Model::get`], over the parts of the model so that the
/// shared summary can call it with the document it is walking.
pub(super) fn get_in(document: &mut Document, tables: &GameTables, field: &FieldId) -> Result<Value, EditError> {
    let binding = Binding::resolve(field)?;
    if let Binding::Character { id, part } = binding {
        if let Part::Spell(chain) = part {
            return spell::level(document, tables, id, chain, field);
        }
        if binding.is_derived() {
            return derived::value(document, tables, id, part, field);
        }
    }
    let path = locate(document, &binding, field)?;
    let read = if binding.is_text() {
        read_text(document, &path).map(Value::Text)
    } else {
        read_int(document, &path).map(Value::Int)
    };
    read.map_err(|source| format_error(field, source))
}
