//! The format of a Pixel Remaster game: detection by slot name and the typed
//! open.

use std::fmt;
use std::marker::PhantomData;

use neoffse_core::{Detection, FileProbe, FormatError, Game, SaveDocument, SaveFiles, SaveFormat};
use neoffse_data::{GameTables, Language, tables_for};

use super::open::open_slot;
use super::{SlotDocument, SlotModel};
use crate::slots;

/// The Pixel Remaster format of `neoffse-core` for the game of `M`.
///
/// It holds the ID tables and the language of the view labels, fixed for
/// every document it opens: a language switch builds a new format. It is
/// `Send + Sync` for every `M`, because it stores no model.
pub struct SlotFormat<M: SlotModel> {
    tables: &'static GameTables,
    language: Language,
    model: PhantomData<fn() -> M>,
}

// Written by hand: a derive would require `M: Copy` and `M: Debug` of a
// type the format never stores.
#[allow(clippy::expl_impl_clone_on_copy, reason = "a derive would bound `M: Copy`, and models are not Copy")]
impl<M: SlotModel> Clone for SlotFormat<M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: SlotModel> Copy for SlotFormat<M> {}

impl<M: SlotModel> fmt::Debug for SlotFormat<M> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SlotFormat")
            .field("game", &M::GAME)
            .field("language", &self.language)
            .finish_non_exhaustive()
    }
}

impl<M: SlotModel> Default for SlotFormat<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: SlotModel> SlotFormat<M> {
    /// A format with English item names.
    ///
    /// # Panics
    ///
    /// Panics when the embedded data holds no tables for the game of `M`,
    /// as [`neoffse_data::tables_for`] does. The tables are a constant of
    /// the build.
    #[must_use]
    pub fn new() -> Self {
        Self::with_language(Language::En)
    }

    /// A format whose views name items in `language`. Field labels and
    /// character names do not follow it.
    ///
    /// # Panics
    ///
    /// Panics when the embedded data holds no tables for the game of `M`,
    /// as [`neoffse_data::tables_for`] does.
    #[must_use]
    pub fn with_language(language: Language) -> Self {
        let Some(tables) = tables_for(M::GAME) else {
            panic!("the embedded data holds no tables for {}", M::GAME.id())
        };
        Self {
            tables,
            language,
            model: PhantomData,
        }
    }

    /// The language of the item names in the views.
    #[must_use]
    pub fn language(&self) -> Language {
        self.language
    }

    /// Opens the one file of `files` as a slot of the game of `M`: decodes
    /// the container, parses the JSON text, applies the game rule and checks
    /// the shape.
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
    /// `OpenError` (`FileCount`, `NotASlot`,
    /// `OtherGame`), a `CodecError` for bytes that are no container, or a
    /// `DocumentError` for text that is no JSON. A list that is missing or
    /// holds a row of another shape, and a repeated `id` or `contentId`
    /// (`EditError::DuplicateRow` as the source of `NotASlot`), are not a
    /// slot.
    pub fn open_document(&self, files: &SaveFiles) -> Result<SlotDocument<M>, FormatError> {
        open_slot(self.tables, self.language, files)
    }
}

impl<M: SlotModel> SaveFormat for SlotFormat<M> {
    fn game(&self) -> Game {
        M::GAME
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
