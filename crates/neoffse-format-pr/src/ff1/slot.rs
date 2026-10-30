//! `Ff1Model` as the model of the shared slot layer. Each method calls the
//! inherent method of the same name: an inherent method wins over a trait
//! method of the same name, so none of them recurses.

use neoffse_core::{
    CapPolicy, Capability, CollectionMeta, Edit, EditError, FieldId, FieldMeta, FormatError, Game, GameChange, RowId, SaveSummary, SaveView, Value,
};
use neoffse_data::{GameTables, Language};

use super::Ff1Model;
use crate::document::Document;
use crate::shared::SlotModel;

impl SlotModel for Ff1Model {
    const GAME: Game = super::GAME;
    const CAPABILITIES: &'static [Capability] = &super::CAPABILITIES;
    const DOCUMENT: &'static str = "Ff1Document";

    fn new(document: Document, tables: &'static GameTables) -> Self {
        Ff1Model::new(document, tables)
    }

    fn document(&self) -> &Document {
        Ff1Model::document(self)
    }

    fn tables(&self) -> &'static GameTables {
        self.tables
    }

    fn rows(&mut self, collection: &FieldId) -> Result<Vec<u32>, EditError> {
        Ff1Model::rows(self, collection)
    }

    fn get(&mut self, field: &FieldId) -> Result<Value, EditError> {
        Ff1Model::get(self, field)
    }

    fn field_meta(&self, field: &FieldId) -> Option<FieldMeta> {
        Ff1Model::field_meta(self, field)
    }

    fn collection_meta(&self, collection: &FieldId) -> Option<CollectionMeta> {
        Ff1Model::collection_meta(self, collection)
    }

    fn summary(&mut self) -> Result<SaveSummary, FormatError> {
        Ff1Model::summary(self)
    }

    fn view(&mut self, language: Language) -> Result<SaveView, FormatError> {
        Ff1Model::view(self, language)
    }

    fn apply(&mut self, edit: &Edit, policy: CapPolicy) -> Result<Edit, EditError> {
        Ff1Model::apply(self, edit, policy)
    }

    fn apply_game(&mut self, edit: &Edit) -> Result<GameChange, EditError> {
        Ff1Model::apply_game(self, edit)
    }

    fn row_locked(&mut self, row: &RowId) -> Result<bool, EditError> {
        Ff1Model::row_locked(self, row)
    }

    fn field_locked(&self, field: &FieldId) -> Result<bool, EditError> {
        Ff1Model::field_locked(self, field)
    }
}
