//! The FF I format: detection by slot name and the typed open.

use std::collections::HashSet;

use neoffse_core::{Detection, EditError, FieldId, FileProbe, FormatError, Game, RowId, SaveDocument, SaveFiles, SaveFormat};
use neoffse_data::{GameTables, Language, tables_for};

use super::save::Ff1Document;
use super::summary::decode_error;
use super::{Ff1Model, GAME, names};
use crate::container::decode;
use crate::document::Document;
use crate::games::{check_markers, game_of_count};
use crate::open_error::OpenError;
use crate::slots;

/// The FF I Pixel Remaster format of `neoffse-core`.
///
/// It holds the ID tables and the language of the view labels, fixed for
/// every document it opens: a language switch builds a new format.
#[derive(Debug, Clone, Copy)]
pub struct Ff1Format {
    tables: &'static GameTables,
    language: Language,
}

impl Ff1Format {
    /// A format with English item names.
    ///
    /// # Panics
    ///
    /// Panics when the embedded FF I tables are invalid, as
    /// [`neoffse_data::tables_for`] does. The tables are a constant of the
    /// build.
    #[must_use]
    pub fn new() -> Self {
        Self::with_language(Language::En)
    }

    /// A format whose views name items in `language`. Field labels and
    /// character names do not follow it.
    ///
    /// # Panics
    ///
    /// Panics when the embedded FF I tables are invalid, as
    /// [`neoffse_data::tables_for`] does.
    #[must_use]
    pub fn with_language(language: Language) -> Self {
        let Some(tables) = tables_for(GAME) else {
            panic!("the embedded data holds no FF I tables")
        };
        Self { tables, language }
    }

    /// The language of the item names in the views.
    #[must_use]
    pub fn language(&self) -> Language {
        self.language
    }

    /// Opens the one file of `files` as an FF I slot: decodes the container,
    /// parses the JSON text, applies the game rule and checks the shape.
    ///
    /// The file may have any name; the document writes it back under that
    /// name. The game rule refuses a slot whose vehicle count or marker keys
    /// belong to another game
    /// (`docs/formats/pixel-remaster.md#telling-the-games-apart`). It rests
    /// on counts measured on one Steam build, so a build that changes a
    /// transportation table makes it refuse that build's slots. The shape
    /// check needs the character list and the two item lists, nothing else,
    /// and each key of a list (`id` or `contentId`) must appear once: the
    /// model looks a row up by key, so a repeated key would make a `Remove`
    /// that its inverse cannot undo.
    ///
    /// # Errors
    ///
    /// `FormatError::Decode` for every failure. Its source is an
    /// [`OpenError`] (`FileCount`, `NotASlot`, `OtherGame`), a `CodecError`
    /// for bytes that are no container, or a `DocumentError` for text that is
    /// no JSON. A list that is missing or holds a row of another shape, and a
    /// repeated `id` or `contentId` (`EditError::DuplicateRow` as the source
    /// of `NotASlot`), are not a slot.
    pub fn open_document(&self, files: &SaveFiles) -> Result<Ff1Document, FormatError> {
        if files.len() != 1 {
            return Err(decode_error(OpenError::FileCount { found: files.len() }));
        }
        let Some((name, bytes)) = files.iter().next() else {
            return Err(decode_error(OpenError::FileCount { found: 0 }));
        };
        let decoded = decode(bytes).map_err(decode_error)?;
        let mut document = Document::parse(&decoded.json).map_err(decode_error)?;
        let (game, vehicles) = game_of_count(&mut document).map_err(decode_error)?;
        if game != GAME {
            return Err(decode_error(OpenError::OtherGame { found: Some(game), vehicles }));
        }
        check_markers(&mut document, GAME, vehicles).map_err(decode_error)?;
        let mut model = Ff1Model::new(document, self.tables);
        for collection in [names::CHARACTER, names::INVENTORY, names::KEY_ITEMS] {
            let not_a_slot = |source| decode_error(OpenError::NotASlot { source: Box::new(source) });
            let keys = model.rows(&FieldId::new(collection)).map_err(not_a_slot)?;
            if let Some(key) = repeated_key(&keys) {
                let row = RowId::new(FieldId::new(collection), key).id().clone();
                return Err(not_a_slot(EditError::DuplicateRow { row }));
            }
        }
        Ok(Ff1Document::new(model, decoded.container, name.to_owned(), self.language))
    }
}

/// The first key that appears twice in `keys`, in document order.
fn repeated_key(keys: &[u32]) -> Option<u32> {
    let mut seen = HashSet::new();
    keys.iter().copied().find(|key| !seen.insert(*key))
}

impl Default for Ff1Format {
    fn default() -> Self {
        Self::new()
    }
}

impl SaveFormat for Ff1Format {
    fn game(&self) -> Game {
        GAME
    }

    /// Claims a slot file by its name through [`slots::detect`]. Never
    /// `Certain`: the head names no game.
    fn detect(&self, probe: &FileProbe<'_>) -> Option<Detection> {
        slots::detect(probe)
    }

    fn open(&self, files: &SaveFiles) -> Result<Box<dyn SaveDocument>, FormatError> {
        Ok(Box::new(self.open_document(files)?))
    }
}
