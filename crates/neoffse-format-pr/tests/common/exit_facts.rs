//! What the edit matrix reads from a save, once: the view, the tree, the
//! tables and the first rows `row_locked` leaves free.

use neoffse_core::{Capability, FieldId, Key, RowId, SaveDocument, SaveView, TableKind, Value};
use neoffse_data::GameTables;
use neoffse_format_pr::ff1::{GAME, names};

use super::edit_subject::Subject;
use super::ff1_save::character_field;

pub(crate) const CHARS: &str = "userData.ownedCharacterList.target";
pub(crate) const INV: &str = "userData.normalOwnedItemList.target";
pub(crate) const KEYS: &str = "userData.importantOwendItemList.target";

/// What `build` reads from the save, once.
pub(super) struct Facts<'a> {
    pub subject: &'a Subject,
    pub view: SaveView,
    pub tables: &'static GameTables,
    /// The first Party entity: its row key, document index (0) and path.
    pub char_id: u32,
    pub char_index: usize,
    pub char_path: String,
    pub characters: usize,
    pub inv_rows: Vec<u32>,
    pub key_rows: Vec<u32>,
    pub inv_len: usize,
    pub keys_len: usize,
    /// The first rows `row_locked` leaves free.
    pub free_item: Option<u32>,
    pub free_key: Option<u32>,
}

/// The rows of `collection` in the view, in order.
fn rows_of(view: &SaveView, collection: &str) -> Vec<u32> {
    view.sections
        .iter()
        .flat_map(|section| &section.entities)
        .filter_map(|entity| match entity.id.keys() {
            [Key::Name(name), Key::Id(key)] if *name == collection => Some(*key),
            _ => None,
        })
        .collect()
}

fn first_free(document: &mut dyn SaveDocument, collection: &'static str, rows: &[u32]) -> Option<u32> {
    rows.iter()
        .copied()
        .find(|key| matches!(document.row_locked(&RowId::new(FieldId::new(collection), *key)), Ok(false)))
}

impl<'a> Facts<'a> {
    pub(super) fn read(subject: &'a Subject) -> Facts<'a> {
        let mut document = subject.open();
        let Ok(view) = document.view() else {
            panic!("{}: view failed", subject.label)
        };
        let Some(party) = view.section(Capability::Party) else {
            panic!("{}: no Party section", subject.label)
        };
        let Some([_, Key::Id(char_id)]) = party.entities.first().map(|entity| entity.id.keys()) else {
            panic!("{}: no character", subject.label)
        };
        let char_id = *char_id;
        let characters = party.entities.len();
        let (inv_rows, key_rows) = (rows_of(&view, names::INVENTORY), rows_of(&view, names::KEY_ITEMS));
        let Some(tables) = neoffse_data::tables_for(GAME) else {
            panic!("no FF I tables")
        };
        Facts {
            subject,
            tables,
            char_id,
            char_index: 0,
            char_path: format!("{CHARS}[0]"),
            characters,
            inv_len: subject.tree.array_len(INV).unwrap_or(0),
            keys_len: subject.tree.array_len(KEYS).unwrap_or(0),
            free_item: first_free(&mut *document, names::INVENTORY, &inv_rows),
            free_key: first_free(&mut *document, names::KEY_ITEMS, &key_rows),
            inv_rows,
            key_rows,
            view,
        }
    }

    pub(super) fn int(&self, field: &FieldId) -> Option<i64> {
        match self.view.find(field)?.1.value {
            Value::Int(number) => Some(number),
            _ => None,
        }
    }

    pub(super) fn field(&self, name: &'static str) -> FieldId {
        character_field(self.char_id, name)
    }

    pub(super) fn inventory(key: u32) -> RowId {
        RowId::new(FieldId::new(names::INVENTORY), key)
    }

    pub(super) fn key_item(key: u32) -> RowId {
        RowId::new(FieldId::new(names::KEY_ITEMS), key)
    }

    /// The first id of `kind` that is not a placeholder and not in `rows`.
    pub(super) fn first_unowned(&self, kind: TableKind, rows: &[u32]) -> Option<u32> {
        let table = self.tables.table(kind)?;
        table.iter().find(|entry| !entry.placeholder && !rows.contains(&entry.id)).map(|entry| entry.id)
    }
}
