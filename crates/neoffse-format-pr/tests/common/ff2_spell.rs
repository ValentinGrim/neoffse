//! Helpers of the FF II spell Level tests: the edit, the document paths one
//! Level edit can change, and an undo and redo history driven the way a
//! `Session` drives a boxed document.

use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, GameChange, SaveDocument, SaveFormat, Value};
use neoffse_format_pr::container::{Container, decode};
use neoffse_format_pr::ff2::{Ff2Format, Ff2Model, names};

use super::ff1_format::{one_file, sav, slot_name};
use super::json_diff::{Change, value};

/// `Set character.<id>.spell.<chain>` to `level`.
pub(crate) fn level_edit(id: u32, chain: u32, level: i64) -> Edit {
    Edit::set(names::spell_field(id, chain), Value::Int(level))
}

/// Replays `set` on the model under `StorageOnly`, the way a session replays
/// an undo or a redo.
pub(crate) fn replay(model: &mut Ff2Model, set: &ChangeSet) -> Result<ChangeSet, EditError> {
    set.apply_with(CapPolicy::StorageOnly, |edit, policy| model.apply(edit, policy))
}

/// Where the places of one spell sit: the character's position in the party
/// list, the entry index in `abilityList`, the index of the key-3 value in
/// the dictionary and the entry index in its list, and the position of the
/// old content id in each order list when it holds it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Where {
    pub character: usize,
    pub list: usize,
    pub copy: usize,
    pub addition: Option<usize>,
    pub sort: Option<usize>,
}

impl Where {
    /// The paths a Level edit changes when it moves `parts` of the two
    /// entries (`abilityId`, `contentId`, `skillLevel`) and the order places.
    /// The key-3 value is the first of the dictionary.
    pub(crate) fn changes(&self, parts: &[&str]) -> Vec<Change> {
        self.changes_of(parts, parts, 0)
    }

    /// As [`changes`](Self::changes), with the parts of each entry apart and
    /// the index of the key-3 value in the dictionary.
    pub(crate) fn changes_of(&self, list_parts: &[&str], copy_parts: &[&str], values: usize) -> Vec<Change> {
        let base = format!("userData.ownedCharacterList.target[{}]", self.character);
        let mut paths = Vec::new();
        for part in list_parts {
            paths.push(format!("{base}.abilityList.target[{}].{part}", self.list));
        }
        for part in copy_parts {
            paths.push(format!("{base}.abilityDictionary.values[{values}].target[{}].{part}", self.copy));
        }
        if let Some(position) = self.addition {
            paths.push(format!("{base}.additionOrderOwnedAbilityIds.target[{position}]"));
        }
        if let Some(position) = self.sort {
            paths.push(format!("{base}.sortOrderOwnedAbilityIds.target[{position}]"));
        }
        paths.iter().map(|path| value(path)).collect()
    }
}

/// A boxed document and the history a session keeps: an entry only after a
/// successful `apply_game`, an undo through `apply_all` under `StorageOnly`.
pub(crate) struct History {
    pub document: Box<dyn SaveDocument>,
    pub undo: Vec<GameChange>,
    pub redo: Vec<GameChange>,
    name: &'static str,
}

impl History {
    /// The slot `text` opened through the trait, under the name of slot 1.
    pub(crate) fn open(text: &str) -> Self {
        let format: &dyn SaveFormat = &Ff2Format::new();
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
