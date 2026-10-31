//! Builds the Slint data of the editor from a save view, and the table that
//! turns the keys the screens send back into field and row ids.
//!
//! The builder names no game and no field. It reads the view's sections,
//! entities and collection metadata, so a format that publishes the same
//! shapes gets the same screens.

use neoffse::Change;
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
    /// The field sits on a page only expert mode edits ([`expert_only`]).
    pub(crate) expert_only: bool,
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
/// ignored. `expert` lifts the locks the game interface sets and shows the
/// pages only it can edit: a page of fields that are all locked and none read
/// only stays out of the default mode, except in the Party section, whose
/// cards open pages by position. UI thread only: the result holds Slint
/// models.
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
    // No save holds 2^31 rows: the saturation never meets a real index.
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
        let collections = row_collections(section);
        let mut pages: Vec<&Entity> = Vec::new();
        let mut rows: Vec<Vec<&Entity>> = vec![Vec::new(); collections.len()];
        for entity in &section.entities {
            match row_list(&collections, entity) {
                Some(index) => rows[index].push(entity),
                None => pages.push(entity),
            }
        }
        let mut every = section.entities.iter().flat_map(|entity| &entity.fields).peekable();
        let read_only = every.peek().is_some() && every.all(|view| view.meta.read_only);
        let hide = hides_expert_only(section.capability);
        let pages: Vec<EntityPage> = pages.into_iter().filter_map(|entity| self.page(entity, hide)).collect();
        let lists: Vec<ItemList> = collections.into_iter().zip(rows).map(|(meta, entities)| self.list(meta, &entities)).collect();
        SectionData {
            present: true,
            read_only,
            pages: model_of(pages),
            lists: model_of(lists),
        }
    }

    /// The page of `entity`, `None` when only expert mode edits it, `hide`
    /// holds and expert mode is off: no key is issued for a page the screen
    /// never shows.
    fn page(&mut self, entity: &Entity, hide: bool) -> Option<EntityPage> {
        let locked: Vec<bool> = entity.fields.iter().map(|view| self.locks.field_locked(&view.id)).collect();
        let expert_only = expert_only(entity, &locked);
        if hide && expert_only && !self.context.expert {
            return None;
        }
        let fields = entity
            .fields
            .iter()
            .zip(locked)
            .map(|(view, locked)| self.field(view, &view.meta.label, locked, expert_only))
            .collect::<Vec<_>>();
        Some(EntityPage {
            label: entity.label.as_str().into(),
            fields: model_of(fields),
        })
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
        let fields = entity
            .fields
            .iter()
            .map(|view| {
                let locked = self.locks.field_locked(&view.id);
                self.field(view, &entity.label, locked, false)
            })
            .collect::<Vec<_>>();
        Some(ItemRow {
            key,
            label: entity.label.as_str().into(),
            fields: model_of(fields),
            locked,
            removable: meta.removable,
            can_remove: meta.removable && (self.context.expert || !locked),
        })
    }

    fn field(&mut self, view: &FieldView, notice_label: &str, locked: bool, expert_only: bool) -> FieldRow {
        let key = key_of(self.layout.fields.len());
        self.layout.fields.push(FieldSlot {
            id: view.id.clone(),
            meta: view.meta.clone(),
            label: notice_label.to_owned(),
            value: view.value.clone(),
            expert_only,
        });
        field_row(&self.context, key, view, locked)
    }
}

/// The collections of `section` whose rows the screens list.
fn row_collections(section: &Section) -> Vec<&CollectionMeta> {
    section.collections.iter().filter(|meta| meta.insertable || meta.removable).collect()
}

/// The position in `collections` of the list `entity` is a row of, `None`
/// when `entity` is a page.
fn row_list(collections: &[&CollectionMeta], entity: &Entity) -> Option<usize> {
    let parent = entity.id.parent();
    collections.iter().position(|meta| parent.as_ref() == Some(&meta.id))
}

/// Whether the page of `entity` holds something only expert mode edits: it
/// has fields, none is read only, and the game interface locks every one
/// (`locked` answers for each field, in order). With nothing to edit in the
/// default mode, the screen shows the page in expert mode alone. A page of
/// read-only fields (FF I's Magic) or with no field stays.
///
/// The builder and [`without_expert_only`] both decide through this
/// function and [`hides_expert_only`], so the pages hidden and the review
/// rows hidden agree.
fn expert_only(entity: &Entity, locked: &[bool]) -> bool {
    !entity.fields.is_empty() && entity.fields.iter().zip(locked).all(|(view, locked)| *locked && !view.meta.read_only)
}

/// Whether the pages of a section can hide as expert-only. The Party pages
/// cannot: the cards open them by their position, so a hidden page would
/// shift every later card onto the wrong character.
fn hides_expert_only(capability: Capability) -> bool {
    capability != Capability::Party
}

/// `changes` without the rows of the pages only expert mode edits, judged on
/// `view` (the current state) with the lock answers of `locks`.
///
/// A change belongs to a page when its capability and entity id match a page
/// of the view: the entities of a collection's rows are no pages, and a row
/// that left the view stays in the list.
pub(crate) fn without_expert_only(changes: Vec<Change>, view: &SaveView, locks: &mut dyn Locks) -> Vec<Change> {
    let mut verdicts: Vec<(Capability, FieldId, bool)> = Vec::new();
    let mut keep = Vec::with_capacity(changes.len());
    for change in changes {
        let known = verdicts
            .iter()
            .find(|(capability, id, _)| *capability == change.capability && *id == change.entity);
        let hidden = if let Some((_, _, hidden)) = known {
            *hidden
        } else {
            let hidden = page_in(view, change.capability, &change.entity).is_some_and(|entity| {
                let locked: Vec<bool> = entity.fields.iter().map(|view| locks.field_locked(&view.id)).collect();
                expert_only(entity, &locked)
            });
            verdicts.push((change.capability, change.entity.clone(), hidden));
            hidden
        };
        if !hidden {
            keep.push(change);
        }
    }
    keep
}

/// The page of `view` with id `id` in the section of `capability`, `None`
/// for a row of a collection or an entity the view lacks.
fn page_in<'a>(view: &'a SaveView, capability: Capability, id: &FieldId) -> Option<&'a Entity> {
    let section = view.section(capability).filter(|section| hides_expert_only(section.capability))?;
    let collections = row_collections(section);
    section
        .entities
        .iter()
        .find(|entity| entity.id == *id && row_list(&collections, entity).is_none())
}
