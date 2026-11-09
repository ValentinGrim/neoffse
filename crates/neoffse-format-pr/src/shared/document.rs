//! The open slot document: the model, the container facts and the file name.

use std::fmt;

use neoffse_core::{
    CapPolicy, Capability, ChangeSet, Edit, EditError, FieldId, FormatError, Game, GameChange, RowId, SaveDocument, SaveFiles, SaveSummary, SaveView,
};
use neoffse_data::Language;

use super::SlotModel;
use crate::container::{Container, decode, encode};
use crate::slots::slot_for_name;

/// A Pixel Remaster slot opened by its format.
///
/// It keeps the BOM and the exact trailer of the opened file and the file
/// name, so `encode` writes the file back as it came. Its `Debug` is written
/// by hand and prints the document name of the model and the file name only,
/// and it has no `Clone`.
pub struct SlotDocument<M: SlotModel> {
    model: M,
    container: Container,
    name: String,
    language: Language,
}

impl<M: SlotModel> SlotDocument<M> {
    pub(crate) fn new(model: M, container: Container, name: String, language: Language) -> Self {
        Self {
            model,
            container,
            name,
            language,
        }
    }

    /// The name of the opened file; `encode` writes the document under it.
    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.name
    }

    /// The BOM and the trailer of the opened file.
    #[must_use]
    pub fn container(&self) -> &Container {
        &self.container
    }
}

/// Prints the document name of the model and the file name, never the
/// content.
impl<M: SlotModel> fmt::Debug for SlotDocument<M> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct(M::DOCUMENT).field("name", &self.name).finish_non_exhaustive()
    }
}

impl<M: SlotModel> SaveDocument for SlotDocument<M> {
    fn game(&self) -> Game {
        M::GAME
    }

    fn capabilities(&self) -> &[Capability] {
        M::CAPABILITIES
    }

    /// The model's summary; a slot the `id` key does not give falls back to
    /// the slot of the file name.
    fn summary(&mut self) -> Result<SaveSummary, FormatError> {
        let mut summary = self.model.summary()?;
        if summary.slot.is_none() {
            summary.slot = slot_for_name(&self.name);
        }
        Ok(summary)
    }

    fn view(&mut self) -> Result<SaveView, FormatError> {
        self.model.view(self.language)
    }

    fn apply(&mut self, edit: &Edit, policy: CapPolicy) -> Result<Edit, EditError> {
        self.model.apply(edit, policy)
    }

    /// The provided body, plus a snapshot of the document for a set of two or
    /// more edits. A rolled-back set rewrites the levels it touched compact;
    /// restoring the snapshot brings the original text back byte for byte.
    fn apply_all(&mut self, changes: &ChangeSet, policy: CapPolicy) -> Result<ChangeSet, EditError> {
        let snapshot = (changes.len() > 1).then(|| self.model.clone());
        let result = changes.apply_with(policy, |edit, policy| self.apply(edit, policy));
        if let (Err(_), Some(model)) = (&result, snapshot) {
            self.model = model;
        }
        result
    }

    /// The model's game interface: caps, ID tables, equipment and inventory
    /// rules.
    fn apply_game(&mut self, edit: &Edit) -> Result<GameChange, EditError> {
        self.model.apply_game(edit)
    }

    fn row_locked(&mut self, row: &RowId) -> Result<bool, EditError> {
        self.model.row_locked(row)
    }

    fn field_locked(&self, field: &FieldId) -> Result<bool, EditError> {
        self.model.field_locked(field)
    }

    fn encode(&self) -> Result<SaveFiles, FormatError> {
        let bytes = encode(&self.model.document().to_json(), &self.container).map_err(|source| FormatError::Encode { source: Box::new(source) })?;
        let mut files = SaveFiles::new();
        files.push(self.name.clone(), bytes)?;
        Ok(files)
    }

    fn verify(&self, files: &SaveFiles) -> Result<(), FormatError> {
        let Some(bytes) = files.get(&self.name) else {
            return Err(FormatError::MissingFile { name: self.name.clone() });
        };
        let decoded = decode(bytes).map_err(|source| FormatError::Decode { source: Box::new(source) })?;
        if decoded.json != self.model.document().to_json() || decoded.container != self.container {
            return Err(FormatError::VerifyMismatch { name: self.name.clone() });
        }
        Ok(())
    }
}
