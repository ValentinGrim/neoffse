//! The seam between the shared slot layer and the model of one game.

use neoffse_core::{
    CapPolicy, Capability, CollectionMeta, Edit, EditError, FieldId, FieldMeta, FormatError, Game, GameChange, RowId, SaveSummary, SaveView, Value,
};
use neoffse_data::{GameTables, Language};

use crate::document::Document;

/// What [`SlotDocument`](super::SlotDocument) and the shared open pipeline
/// need from the model of one Pixel Remaster game.
///
/// A model owns the lossless [`Document`] of a slot and the ID tables of its
/// game, and maps both onto the field vocabulary of its game. Each method
/// has the contract of the inherent method of the same name on the model of
/// the first game.
pub trait SlotModel: Clone + Send + 'static {
    /// The game this model reads.
    const GAME: Game;

    /// The capabilities of a slot of this game.
    const CAPABILITIES: &'static [Capability];

    /// The name `Debug` prints for a document of this model: the name of the
    /// alias of `SlotDocument` that the game module exports.
    const DOCUMENT: &'static str;

    /// Wraps `document`. Nothing is validated: a document of another shape
    /// fails when a field reads it.
    fn new(document: Document, tables: &'static GameTables) -> Self;

    /// The document, for `to_json`.
    fn document(&self) -> &Document;

    /// The ID tables of the game.
    fn tables(&self) -> &'static GameTables;

    /// The row keys of `collection` in document order.
    ///
    /// # Errors
    ///
    /// `NotACollection` when `collection` is none the model declares, `Format`
    /// when the document does not hold the list or a key.
    fn rows(&mut self, collection: &FieldId) -> Result<Vec<u32>, EditError>;

    /// The value of `field`.
    ///
    /// # Errors
    ///
    /// `UnknownField` for an undeclared path, `MissingRow` for an absent
    /// row, `Format` when the document holds another shape.
    fn get(&mut self, field: &FieldId) -> Result<Value, EditError>;

    /// The metadata of `field`, or `None` for an undeclared path.
    fn field_meta(&self, field: &FieldId) -> Option<FieldMeta>;

    /// The metadata of `collection`, or `None` for any other path.
    fn collection_meta(&self, collection: &FieldId) -> Option<CollectionMeta>;

    /// The slot, play time, place and party, without writing.
    ///
    /// # Errors
    ///
    /// `FormatError::Decode` when the document holds a shape the summary
    /// cannot read.
    fn summary(&mut self) -> Result<SaveSummary, FormatError>;

    /// A snapshot of every entity the editor shows, item names in
    /// `language`, without writing.
    ///
    /// # Errors
    ///
    /// `FormatError::Decode` when the document holds a shape the view cannot
    /// read.
    fn view(&mut self, language: Language) -> Result<SaveView, FormatError>;

    /// The raw edit: storage bounds, and under `CapPolicy::Enforce` the caps
    /// and the ID tables, with no game rule. Returns the inverse.
    ///
    /// # Errors
    ///
    /// The `EditError` of the edit; a failed call leaves the document as it
    /// was.
    fn apply(&mut self, edit: &Edit, policy: CapPolicy) -> Result<Edit, EditError>;

    /// The edit the way the game would make it, with its rules.
    ///
    /// # Errors
    ///
    /// The `EditError` of the edit; a failed call leaves the document as it
    /// was.
    fn apply_game(&mut self, edit: &Edit) -> Result<GameChange, EditError>;

    /// True when the game interface refuses to remove `row`, whatever the
    /// values.
    ///
    /// # Errors
    ///
    /// `Format` when the document does not hold the rows the rule reads.
    fn row_locked(&mut self, row: &RowId) -> Result<bool, EditError>;

    /// True when the game interface refuses to set `field`, whatever the
    /// value.
    ///
    /// # Errors
    ///
    /// `UnknownField` for an undeclared path.
    fn field_locked(&self, field: &FieldId) -> Result<bool, EditError>;
}
