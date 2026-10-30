//! Helpers for the editor view model: an open synthetic save, finders that
//! look rows up by label, a fake lock source.

use std::sync::{Arc, Mutex, PoisonError};

use neoffse::Session;
use neoffse::core::{FieldId, Game, RowId, TableKind};
use neoffse::data::{Language, tables_for};
use neoffse_app::editor::{Input, Locks};
use neoffse_app::home::OpenOutcome;
use neoffse_app::{EditorData, EditorViewModel, EntityPage, FieldRow, ItemList, ItemRow, ReviewKind, ReviewRow, SectionData};

use super::{Flow, ID_1, Outcomes, Site, items, slot_bytes, slot_file, steam_slot};

/// The three sections of the editor.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Part {
    Party,
    Inventory,
    Magic,
}

/// A site with the synthetic FF I slot 1 in a Proton save folder.
pub(crate) fn editing_site() -> Site {
    let site = Site::new();
    steam_slot(&site, 0, ID_1, slot_file(1), &slot_bytes(1));
    site
}

/// A flow whose first synthetic slot is open.
pub(crate) struct Editing {
    pub(crate) flow: Flow,
}

/// Opens slot 1 of `site`, run to the end.
pub(crate) fn editing(site: &Site) -> Editing {
    let flow = Flow::new(site, None);
    let outcomes = Outcomes::new();
    flow.click(slot_file(1), &outcomes);
    flow.manual.run_all();
    assert_eq!(outcomes.outcomes(), vec![OpenOutcome::Opened]);
    let editing = Editing { flow };
    editing.editor().data().expect("the first data call");
    editing
}

impl Editing {
    pub(crate) fn editor(&self) -> &EditorViewModel {
        &self.flow.editor
    }

    pub(crate) fn session(&self) -> Arc<Mutex<Session>> {
        self.flow.state.session().expect("a save is open")
    }

    /// Runs `action` on the locked session.
    pub(crate) fn with_session<T>(&self, action: impl FnOnce(&mut Session) -> T) -> T {
        let session = self.session();
        let mut guard = session.lock().unwrap_or_else(PoisonError::into_inner);
        action(&mut guard)
    }

    pub(crate) fn pending(&self) -> usize {
        self.with_session(|session| session.pending_count())
    }

    pub(crate) fn undo(&self) {
        assert!(self.with_session(Session::undo).expect("undo"), "nothing to undo");
    }

    pub(crate) fn data(&self) -> EditorData {
        self.editor().data().expect("data")
    }

    /// The text of the gil field.
    pub(crate) fn gil(&self) -> String {
        field(&self.data(), Part::Inventory, "Gil", "Gil").text.to_string()
    }

    /// Commits `value` to the gil field and asserts the commit raised no notice.
    pub(crate) fn set_gil(&self, value: &str) {
        let key = field(&self.data(), Part::Inventory, "Gil", "Gil").key;
        assert!(self.editor().commit(key, Input::Text(value.to_owned())).is_none());
    }
}

pub(crate) fn section(data: &EditorData, part: Part) -> SectionData {
    match part {
        Part::Party => data.party.clone(),
        Part::Inventory => data.inventory.clone(),
        Part::Magic => data.magic.clone(),
    }
}

pub(crate) fn pages(section: &SectionData) -> Vec<EntityPage> {
    items(&section.pages)
}

pub(crate) fn lists(section: &SectionData) -> Vec<ItemList> {
    items(&section.lists)
}

pub(crate) fn fields(page: &EntityPage) -> Vec<FieldRow> {
    items(&page.fields)
}

pub(crate) fn rows(list: &ItemList) -> Vec<ItemRow> {
    items(&list.rows)
}

/// Every field row of `data`, in traversal order: per section the pages, then
/// the rows of each list.
pub(crate) fn all_fields(data: &EditorData) -> Vec<FieldRow> {
    let mut out = Vec::new();
    for part in [Part::Party, Part::Inventory, Part::Magic] {
        let section = section(data, part);
        for page in pages(&section) {
            out.extend(fields(&page));
        }
        for list in lists(&section) {
            for row in rows(&list) {
                out.extend(items(&row.fields));
            }
        }
    }
    out
}

pub(crate) fn page(data: &EditorData, part: Part, label: &str) -> EntityPage {
    pages(&section(data, part))
        .into_iter()
        .find(|page| page.label == label)
        .unwrap_or_else(|| panic!("no {part:?} page labelled {label}"))
}

/// The field `field_label` on the page `page_label` of `part`.
pub(crate) fn field(data: &EditorData, part: Part, page_label: &str, field_label: &str) -> FieldRow {
    fields(&page(data, part, page_label))
        .into_iter()
        .find(|row| row.label == field_label)
        .unwrap_or_else(|| panic!("no field labelled {field_label} on page {page_label}"))
}

pub(crate) fn list(data: &EditorData, title: &str) -> ItemList {
    lists(&data.inventory)
        .into_iter()
        .find(|list| list.title == title)
        .unwrap_or_else(|| panic!("no list titled {title}"))
}

/// The label the FF I view gives the row of content `id`: the English name,
/// else `<fallback> <id>`.
pub(crate) fn item_label(id: u32, fallback: &str) -> String {
    let tables = tables_for(Game::Ff1Pr).expect("FF I tables");
    tables
        .content(id)
        .and_then(|entry| entry.name(Language::En))
        .filter(|name| !name.is_empty())
        .map_or_else(|| format!("{fallback} {id}"), str::to_owned)
}

/// The row of content `id` in the list titled `title`.
pub(crate) fn row_of(data: &EditorData, title: &str, id: u32) -> ItemRow {
    let fallback = if title == "Key items" { "Key item" } else { "Item" };
    let label = item_label(id, fallback);
    rows(&list(data, title))
        .into_iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| panic!("no row for content {id} in {title}"))
}

/// The count field of an item row.
pub(crate) fn count_of(row: &ItemRow) -> FieldRow {
    items(&row.fields).into_iter().next().expect("a count field")
}

/// A non-placeholder weapon id the inventory of `data` does not hold.
pub(crate) fn spare_weapon(data: &EditorData) -> u32 {
    let tables = tables_for(Game::Ff1Pr).unwrap();
    let labels: Vec<String> = rows(&list(data, "Items")).iter().map(|row| row.label.to_string()).collect();
    tables
        .table(TableKind::Weapon)
        .unwrap()
        .iter()
        .filter(|entry| !entry.placeholder)
        .map(|entry| entry.id)
        .find(|id| !labels.contains(&item_label(*id, "Item")))
        .expect("a weapon the fixture does not hold")
}

/// A review row of `kind` for `entity`, `label` and the two texts.
pub(crate) fn review_row(kind: ReviewKind, entity: &str, label: &str, before: &str, after: &str) -> ReviewRow {
    ReviewRow {
        kind,
        entity: entity.into(),
        label: label.into(),
        before: before.into(),
        after: after.into(),
    }
}

/// A lock source that answers one fixed value and records its questions.
#[derive(Debug, Default)]
pub(crate) struct FakeLocks {
    pub(crate) fields: bool,
    pub(crate) rows: bool,
    pub(crate) asked_fields: Vec<FieldId>,
    pub(crate) asked_rows: Vec<RowId>,
}

impl FakeLocks {
    /// A source that answers `locked` to every question.
    pub(crate) fn answering(locked: bool) -> Self {
        Self {
            fields: locked,
            rows: locked,
            ..Self::default()
        }
    }
}

impl Locks for FakeLocks {
    fn field_locked(&mut self, field: &FieldId) -> bool {
        self.asked_fields.push(field.clone());
        self.fields
    }

    fn row_locked(&mut self, row: &RowId) -> bool {
        self.asked_rows.push(row.clone());
        self.rows
    }
}
