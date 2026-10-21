//! Builds the Slint data of the editor from a save view, and the table that
//! turns the keys the screens send back into field and row ids.
//!
//! The builder names no game and no field. It reads the view's sections,
//! entities and collection metadata, so a format that publishes the same
//! shapes gets the same screens.

use neoffse::core::{Capability, CollectionMeta, Entity, FieldId, FieldMeta, FieldView, Game, Key, RowId, SaveView, Section, Value};
use neoffse::data::{GameTables, Language, tables_for};

use super::field_row::{FieldContext, field_row, model_of};
use crate::{EditorData, EntityPage, FieldRow, ItemList, ItemRow, SectionData};

/// The locks the builder asks about. A session answers in production; a test
/// fakes it.
pub trait Locks {
    /// Whether the default mode refuses every edit of `field`. A failed query
    /// counts as locked.
    fn field_locked(&mut self, field: &FieldId) -> bool;

    /// Whether the default mode refuses to remove `row`. A failed query
    /// counts as locked.
    fn row_locked(&mut self, row: &RowId) -> bool;
}

/// What a field key stands for.
#[derive(Debug, Clone)]
pub(crate) struct FieldSlot {
    pub(crate) id: FieldId,
    pub(crate) meta: FieldMeta,
    /// The name a notice gives the field: the field's label on a page, the
    /// row's label inside a list.
    pub(crate) label: String,
    pub(crate) value: Value,
}

/// What a row key stands for.
#[derive(Debug, Clone)]
pub(crate) struct RowSlot {
    pub(crate) row: RowId,
    pub(crate) label: String,
}

/// The table behind the keys of one [`EditorData`].
///
/// `Debug` prints save content (labels and values): never log it.
#[derive(Debug, Default)]
pub(crate) struct Layout {
    pub(crate) tables: Option<&'static GameTables>,
    pub(crate) fields: Vec<FieldSlot>,
    pub(crate) rows: Vec<RowSlot>,
    pub(crate) lists: Vec<CollectionMeta>,
}

/// The Slint data for a view, and the table that turns its keys back into
/// ids.
///
/// `Debug` prints save content: never log it.
#[derive(Debug)]
#[non_exhaustive]
pub struct Built {
    /// What the screens show.
    pub data: EditorData,
    pub(crate) layout: Layout,
}

/// Builds the editor data from `view`.
///
/// The three sections follow the capabilities Party, Inventory and Magic; a
/// section the view lacks is absent, and a section of another capability is
/// ignored. `expert` lifts the locks the game interface sets. UI thread only:
/// the result holds Slint models.
#[must_use]
pub fn build(view: &SaveView, game: Game, language: Language, expert: bool, locks: &mut dyn Locks) -> Built {
    let mut builder = Builder {
        context: FieldContext {
            tables: tables_for(game),
            language,
            expert,
        },
        locks,
        layout: Layout {
            tables: tables_for(game),
            ..Layout::default()
        },
    };
    let data = EditorData {
        party: builder.section(view.section(Capability::Party)),
        inventory: builder.section(view.section(Capability::Inventory)),
        magic: builder.section(view.section(Capability::Magic)),
    };
    Built { data, layout: builder.layout }
}

/// A running index as a Slint `int`.
fn key_of(index: usize) -> i32 {
    i32::try_from(index).unwrap_or(i32::MAX)
}

/// The key of the row an entity stands for: its last key, when it is an id.
fn row_key(entity: &Entity) -> Option<u32> {
    match entity.id.keys().last() {
        Some(Key::Id(key)) => Some(*key),
        _ => None,
    }
}

struct Builder<'a> {
    context: FieldContext,
    locks: &'a mut dyn Locks,
    layout: Layout,
}

impl Builder<'_> {
    fn section(&mut self, section: Option<&Section>) -> SectionData {
        let Some(section) = section else {
            return SectionData::default();
        };
        let collections: Vec<&CollectionMeta> = section.collections.iter().filter(|meta| meta.insertable || meta.removable).collect();
        let mut pages: Vec<&Entity> = Vec::new();
        let mut rows: Vec<Vec<&Entity>> = vec![Vec::new(); collections.len()];
        for entity in &section.entities {
            let parent = entity.id.parent();
            match collections.iter().position(|meta| parent.as_ref() == Some(&meta.id)) {
                Some(index) => rows[index].push(entity),
                None => pages.push(entity),
            }
        }
        let mut every = section.entities.iter().flat_map(|entity| &entity.fields).peekable();
        let read_only = every.peek().is_some() && every.all(|view| view.meta.read_only);
        let pages: Vec<EntityPage> = pages.into_iter().map(|entity| self.page(entity)).collect();
        let lists: Vec<ItemList> = collections.into_iter().zip(rows).map(|(meta, entities)| self.list(meta, &entities)).collect();
        SectionData {
            present: true,
            read_only,
            pages: model_of(pages),
            lists: model_of(lists),
        }
    }

    fn page(&mut self, entity: &Entity) -> EntityPage {
        let fields = entity.fields.iter().map(|view| self.field(view, &view.meta.label)).collect::<Vec<_>>();
        EntityPage {
            label: entity.label.as_str().into(),
            fields: model_of(fields),
        }
    }

    fn list(&mut self, meta: &CollectionMeta, entities: &[&Entity]) -> ItemList {
        let key = key_of(self.layout.lists.len());
        self.layout.lists.push(meta.clone());
        let rows: Vec<ItemRow> = entities.iter().filter_map(|entity| self.item_row(meta, entity)).collect();
        ItemList {
            key,
            title: meta.label.as_str().into(),
            rows: model_of(rows),
            insertable: meta.insertable,
        }
    }

    fn item_row(&mut self, meta: &CollectionMeta, entity: &Entity) -> Option<ItemRow> {
        let row = RowId::new(meta.id.clone(), row_key(entity)?);
        let locked = self.locks.row_locked(&row);
        let key = key_of(self.layout.rows.len());
        self.layout.rows.push(RowSlot {
            row,
            label: entity.label.clone(),
        });
        let fields = entity.fields.iter().map(|view| self.field(view, &entity.label)).collect::<Vec<_>>();
        Some(ItemRow {
            key,
            label: entity.label.as_str().into(),
            fields: model_of(fields),
            locked,
            removable: meta.removable,
            can_remove: meta.removable && (self.context.expert || !locked),
        })
    }

    fn field(&mut self, view: &FieldView, notice_label: &str) -> FieldRow {
        let key = key_of(self.layout.fields.len());
        self.layout.fields.push(FieldSlot {
            id: view.id.clone(),
            meta: view.meta.clone(),
            label: notice_label.to_owned(),
            value: view.value.clone(),
        });
        let locked = self.locks.field_locked(&view.id);
        field_row(&self.context, key, view, locked)
    }
}
