//! The undo and redo history of a boxed document, driven the way a `Session`
//! drives it. Any game's format takes the same calls.

use neoffse_core::{CapPolicy, Edit, EditError, GameChange, SaveDocument, SaveFormat};
use neoffse_format_pr::container::{Container, decode};

use super::pr_files::{one_file, sav, slot_name};

/// A boxed document and the history a session keeps: an entry only after a
/// successful `apply_game`, an undo through `apply_all` under `StorageOnly`.
pub(crate) struct History {
    pub document: Box<dyn SaveDocument>,
    pub undo: Vec<GameChange>,
    pub redo: Vec<GameChange>,
    name: &'static str,
}

impl History {
    /// The slot `text` opened through `format`, under the name of slot 1.
    pub(crate) fn open(format: &dyn SaveFormat, text: &str) -> Self {
        let files = one_file(slot_name(1), sav(text, &Container::game()));
        Self {
            document: format.open(&files).unwrap(),
            undo: Vec::new(),
            redo: Vec::new(),
            name: slot_name(1),
        }
    }

    /// The JSON text of the encoded document, decoded independently.
    pub(crate) fn text(&self) -> String {
        decode(self.document.encode().unwrap().get(self.name).unwrap()).unwrap().json
    }

    /// `Session::edit`.
    pub(crate) fn edit(&mut self, edit: &Edit) -> Result<(), EditError> {
        let change = self.document.apply_game(edit)?;
        self.undo.push(change);
        self.redo.clear();
        Ok(())
    }

    /// `Session::undo`; true when an entry was undone.
    pub(crate) fn undo(&mut self) -> bool {
        let Some(change) = self.undo.pop() else { return false };
        self.document.apply_all(&change.inverse, CapPolicy::StorageOnly).unwrap();
        self.redo.push(change);
        true
    }

    /// `Session::redo`; true when an entry was redone.
    pub(crate) fn redo(&mut self) -> bool {
        let Some(change) = self.redo.pop() else { return false };
        self.document.apply_all(&change.forward, CapPolicy::StorageOnly).unwrap();
        self.undo.push(change);
        true
    }
}
