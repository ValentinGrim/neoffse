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

    /// The keyed lists of a slot of this game, as collection names: the
    /// open refuses a slot where one of them holds a key twice.
    ///
    /// A `Remove` of a repeated key returns an `Insert` that fails with
    /// `DuplicateRow`, so an undo of it would fail on every call. Each game
    /// names every list it declares whose rows carry a key.
    const KEYED_LISTS: &'static [&'static str];

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

/// Expands to the `impl SlotModel` of a game model, each method calling the inherent method of the same name, so a game states its constants and writes no delegating body.
macro_rules! impl_slot_model {
    ($model:ty, game: $game:expr, capabilities: $capabilities:expr, keyed_lists: $keyed_lists:expr, document: $document:literal) => {
        impl $crate::shared::SlotModel for $model {
            const GAME: ::neoffse_core::Game = $game;
            const CAPABILITIES: &'static [::neoffse_core::Capability] = &$capabilities;
            const KEYED_LISTS: &'static [&'static str] = &$keyed_lists;
            const DOCUMENT: &'static str = $document;

            fn new(document: $crate::document::Document, tables: &'static ::neoffse_data::GameTables) -> Self {
                <$model>::new(document, tables)
            }

            fn document(&self) -> &$crate::document::Document {
                <$model>::document(self)
            }

            fn tables(&self) -> &'static ::neoffse_data::GameTables {
                self.tables
            }

            fn rows(&mut self, collection: &::neoffse_core::FieldId) -> Result<Vec<u32>, ::neoffse_core::EditError> {
                <$model>::rows(self, collection)
            }

            fn get(&mut self, field: &::neoffse_core::FieldId) -> Result<::neoffse_core::Value, ::neoffse_core::EditError> {
                <$model>::get(self, field)
            }

            fn field_meta(&self, field: &::neoffse_core::FieldId) -> Option<::neoffse_core::FieldMeta> {
                <$model>::field_meta(self, field)
            }

            fn collection_meta(&self, collection: &::neoffse_core::FieldId) -> Option<::neoffse_core::CollectionMeta> {
                <$model>::collection_meta(self, collection)
            }

            fn summary(&mut self) -> Result<::neoffse_core::SaveSummary, ::neoffse_core::FormatError> {
                <$model>::summary(self)
            }

            fn view(&mut self, language: ::neoffse_data::Language) -> Result<::neoffse_core::SaveView, ::neoffse_core::FormatError> {
                <$model>::view(self, language)
            }

            fn apply(&mut self, edit: &::neoffse_core::Edit, policy: ::neoffse_core::CapPolicy) -> Result<::neoffse_core::Edit, ::neoffse_core::EditError> {
                <$model>::apply(self, edit, policy)
            }

            fn apply_game(&mut self, edit: &::neoffse_core::Edit) -> Result<::neoffse_core::GameChange, ::neoffse_core::EditError> {
                <$model>::apply_game(self, edit)
            }

            fn row_locked(&mut self, row: &::neoffse_core::RowId) -> Result<bool, ::neoffse_core::EditError> {
                <$model>::row_locked(self, row)
            }

            fn field_locked(&self, field: &::neoffse_core::FieldId) -> Result<bool, ::neoffse_core::EditError> {
                <$model>::field_locked(self, field)
            }
        }
    };
}

pub(crate) use impl_slot_model;
