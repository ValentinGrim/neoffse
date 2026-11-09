//! FF II edits saved through the editor over a copy of the owner's FF II
//! saves (`NEOFFSE_SAVES`, read only). Every slot is copied into a temporary
//! folder first and saved into it with a temporary data folder. In default
//! mode the test edits a spell Level (a `game_edit` field), adds a keyword and
//! edits the gil; in expert mode it edits one raw spell place and removes a
//! keyword. The saved copy, reopened by the facade, must equal a reference
//! session opened on the original with the same edits applied through
//! `Session::edit` and `Session::edit_expert`, and the backup must equal the
//! original bytes. A target comes from the slot's own view, and an edit with
//! no target in a slot is skipped with a report line. The raw place is a
//! proficiency, the one place kind whose edit leaves the field count of the
//! view alone. A failure names a slot number and a field id, never a value, a
//! file name, a path or a keyword id.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{
    Editing, Flow, Outcomes, Part, PrivateSlot, assert_same, ff2_registry, field, fields, lists, must, must_some, pages, private_slot_copies, rows,
    saving_over, session_of,
};
use neoffse::core::{Capability, Edit, Entity, FieldId, Game, Key, RowId, SaveView, Shape, TableKind, Value};
use neoffse::data::{Language, tables_for};
use neoffse::io::dirs::DataDir;
use neoffse::pr::ff2::names;
use neoffse::{Registry, Running, Session};
use neoffse_app::editor::{Input, SaveOutcome};
use neoffse_app::home::OpenOutcome;
use neoffse_app::{EditorData, EditorViewModel, EntityPage, ItemList};
use neoffse_testkit::report;

/// A field to edit and the position of its row on the editor's page: the page
/// index among the pages of its section, the row index on the page.
struct Target {
    id: FieldId,
    page: usize,
    row: usize,
    label: String,
    value: i64,
}

/// A raw page entity: `character.<id>.raw_spells`.
fn is_raw(entity: &Entity) -> bool {
    matches!(entity.id.keys().last(), Some(Key::Name(name)) if *name == names::RAW_SPELLS)
}

/// The first spell Level below the top of its shape, raised by one. The Level
/// entities lead the Magic section in the order of the editor's pages.
fn level_target(view: &SaveView) -> Option<Target> {
    let magic = view.section(Capability::Magic)?;
    magic
        .entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| !is_raw(entity))
        .find_map(|(page, entity)| {
            entity
                .fields
                .iter()
                .enumerate()
                .find_map(|(row, field)| match (field.meta.game_edit, field.meta.shape, &field.value) {
                    (true, Shape::Int { max, .. }, Value::Int(level)) if *level < max => Some(Target {
                        id: field.id.clone(),
                        page,
                        row,
                        label: field.meta.label.clone(),
                        value: level + 1,
                    }),
                    _ => None,
                })
        })
}

/// The first proficiency place of a raw page that `before` holds with the
/// same value (the Level edit did not move it), moved by one inside its shape.
fn raw_target(view: &SaveView, before: &SaveView) -> Option<Target> {
    let magic = view.section(Capability::Magic)?;
    magic.entities.iter().filter(|entity| is_raw(entity)).enumerate().find_map(|(page, entity)| {
        entity.fields.iter().enumerate().find_map(|(row, field)| {
            let proficiency = matches!(field.id.keys().last(), Some(Key::Name(name)) if *name == names::PROFICIENCY);
            let untouched = before.find(&field.id).is_some_and(|(_, old)| old.value == field.value);
            let (Value::Int(value), Shape::Int { min, max }) = (&field.value, field.meta.shape) else {
                return None;
            };
            let moved = if *value < max { value + 1 } else { value - 1 };
            (proficiency && untouched && !field.meta.read_only && (min..=max).contains(&moved)).then(|| Target {
                id: field.id.clone(),
                page,
                row,
                label: field.meta.label.clone(),
                value: moved,
            })
        })
    })
}

/// The keyword ids of the Inventory section, in document order.
fn keywords_of(view: &SaveView) -> Vec<u32> {
    let entities = view
        .section(Capability::Inventory)
        .map(|section| section.entities.as_slice())
        .unwrap_or_default();
    entities
        .iter()
        .filter_map(|entity| match entity.id.keys() {
            [Key::Name(name), Key::Id(key)] if *name == names::KEYWORDS => Some(*key),
            _ => None,
        })
        .collect()
}

/// The first keyword of the table that `held` lacks.
fn absent_keyword(held: &[u32]) -> Option<u32> {
    let table = tables_for(Game::Ff2Pr)?.table(TableKind::Keyword)?;
    table
        .iter()
        .filter(|entry| !entry.placeholder)
        .map(|entry| entry.id)
        .find(|id| !held.contains(id))
}

fn int_at(view: &SaveView, id: &FieldId) -> Option<i64> {
    match view.find(id)?.1.value {
        Value::Int(number) => Some(number),
        _ => None,
    }
}

fn keyword_row(id: u32) -> RowId {
    RowId::new(FieldId::new(names::KEYWORDS), id)
}

/// The editor over one slot and the reference session that takes the same
/// edits. Each step edits through both, and returns what it applied.
struct Run<'a> {
    slot: usize,
    editor: &'a EditorViewModel,
    reference: Session,
}

impl Run<'_> {
    fn view(&mut self) -> SaveView {
        must(self.reference.view(), &format!("slot {}: the reference view failed", self.slot))
    }

    fn data(&self) -> EditorData {
        must(self.editor.data(), &format!("slot {}: the editor data failed", self.slot))
    }

    fn skip(&self, edit: &str) {
        report(&format!("slot {}: no target for the {edit} edit, skipped", self.slot));
    }

    fn apply(&mut self, edit: &Edit, expert: bool, what: &str) {
        let result = if expert {
            self.reference.edit_expert(edit)
        } else {
            self.reference.edit(edit)
        };
        must(result, &format!("slot {}: the reference {what} edit failed", self.slot));
    }

    /// The key of the field row `target` names on `pages`.
    fn key_of(&self, pages: &[EntityPage], target: &Target) -> i32 {
        let slot = self.slot;
        let page = must_some(pages.get(target.page), &format!("slot {slot}: no editor page for {}", target.id));
        let row = must_some(
            fields(page).into_iter().nth(target.row),
            &format!("slot {slot}: no editor row for {}", target.id),
        );
        assert!(row.label == target.label, "slot {slot}: the editor row of {} has another label", target.id);
        row.key
    }

    fn commit(&self, key: i32, value: i64, what: &str) {
        let notice = self.editor.commit(key, Input::Text(value.to_string()));
        assert!(notice.is_none(), "slot {}: the {what} edit raised a notice", self.slot);
    }

    /// Edits `target` through the editor and the reference.
    fn set(&mut self, key: i32, target: &Target, expert: bool, what: &str) -> (FieldId, i64) {
        self.commit(key, target.value, what);
        self.apply(&Edit::set(target.id.clone(), Value::Int(target.value)), expert, what);
        (target.id.clone(), target.value)
    }

    fn level(&mut self) -> Option<(FieldId, i64)> {
        let Some(target) = level_target(&self.view()) else {
            self.skip("level");
            return None;
        };
        let key = self.key_of(&pages(&self.data().magic), &target);
        Some(self.set(key, &target, false, "level"))
    }

    fn gil(&mut self) -> Option<(FieldId, i64)> {
        let id = FieldId::new(names::GIL);
        let view = self.view();
        let (Some((_, gil)), Some(old)) = (view.find(&id), int_at(&view, &id)) else {
            self.skip("gil");
            return None;
        };
        let value = if gil.meta.cap.is_none_or(|cap| old < cap) { old + 1 } else { old - 1 };
        let key = field(&self.data(), Part::Inventory, "Gil", "Gil").key;
        self.commit(key, value, "gil");
        self.apply(&Edit::set(id.clone(), Value::Int(value)), false, "gil");
        Some((id, value))
    }

    fn raw(&mut self, before: &SaveView) -> Option<(FieldId, i64)> {
        let Some(target) = raw_target(&self.view(), before) else {
            self.skip("raw place");
            return None;
        };
        let key = self.key_of(&pages(&self.data().raw_magic), &target);
        Some(self.set(key, &target, true, "raw place"))
    }

    fn keywords_list(&self) -> ItemList {
        let slot = self.slot;
        let list = lists(&self.data().inventory).into_iter().find(|list| list.title == "Keywords");
        must_some(list, &format!("slot {slot}: no keyword list"))
    }

    fn keyword_add(&mut self) -> Option<u32> {
        let Some(id) = absent_keyword(&keywords_of(&self.view())) else {
            self.skip("keyword add");
            return None;
        };
        let entry = must(i32::try_from(id), "a keyword id fits an i32");
        let notice = self.editor.add_row(self.keywords_list().key, entry);
        assert!(notice.is_none(), "slot {}: the keyword add raised a notice", self.slot);
        self.apply(&Edit::insert(keyword_row(id), None, Vec::new()), false, "keyword add");
        Some(id)
    }

    fn keyword_remove(&mut self) -> Option<u32> {
        let Some(first) = keywords_of(&self.view()).first().copied() else {
            self.skip("keyword remove");
            return None;
        };
        let row = must_some(rows(&self.keywords_list()).into_iter().next(), &format!("slot {}: no keyword row", self.slot));
        assert!(
            self.editor.remove_row(row.key).is_none(),
            "slot {}: the keyword remove raised a notice",
            self.slot
        );
        self.apply(&Edit::remove(keyword_row(first)), true, "keyword remove");
        Some(first)
    }
}

/// How many edits of each kind the run applied.
#[derive(Default)]
struct Counts {
    saved: usize,
    level: usize,
    keyword_add: usize,
    plain: usize,
    raw: usize,
    keyword_remove: usize,
}

/// Opens `slot`, edits it, saves it and compares the result with the reference.
fn save_and_compare(slot: usize, private: PrivateSlot, counts: &mut Counts) {
    let PrivateSlot { name, site, copy } = private;
    let original = must(fs::read(&copy), &format!("slot {slot}: the copy does not read"));
    let flow = Flow::with_registry(&site, None, ff2_registry());
    let outcomes = Outcomes::new();
    flow.click(&name, &outcomes);
    flow.manual.run_all();
    assert!(outcomes.outcomes() == vec![OpenOutcome::Opened], "slot {slot}: did not open");
    let saving = saving_over(Editing { flow }, copy.clone());

    let reference = must(tempfile::tempdir(), "no temporary folder");
    let reference_file = reference.path().join(&name);
    must(fs::write(&reference_file, &original), &format!("slot {slot}: the reference copy failed"));
    let mut run = Run {
        slot,
        editor: saving.ed.editor(),
        reference: session_of(Game::Ff2Pr, &reference_file),
    };
    let before = run.view();

    let mut expected: Vec<(FieldId, i64)> = Vec::new();
    let level = run.level();
    counts.level += usize::from(level.is_some());
    let added = run.keyword_add();
    counts.keyword_add += usize::from(added.is_some());
    let plain = run.gil();
    counts.plain += usize::from(plain.is_some());
    run.editor.set_expert(true);
    let raw = run.raw(&before);
    counts.raw += usize::from(raw.is_some());
    let removed = run.keyword_remove();
    counts.keyword_remove += usize::from(removed.is_some());
    expected.extend([level, plain, raw].into_iter().flatten());
    if expected.is_empty() && added.is_none() && removed.is_none() {
        report(&format!("slot {slot}: no edit applies, not saved"));
        return;
    }
    assert!(run.editor.is_dirty(), "slot {slot}: the edits left the session clean");

    saving.save(Running::Refuse);
    saving.ed.flow.manual.run_all();
    assert!(matches!(saving.outcomes().as_slice(), [SaveOutcome::Saved { .. }]), "slot {slot}: not saved");
    counts.saved += 1;

    let mut reopened = session_of(Game::Ff2Pr, &copy);
    let after = must(reopened.view(), &format!("slot {slot}: the saved view failed"));
    let reference_view = run.view();
    assert_same(slot, &after, &reference_view, &before, None);
    for (id, value) in &expected {
        assert!(int_at(&after, id) == Some(*value), "slot {slot}: the saved {id} differs from the edit");
    }
    let mut keywords = keywords_of(&before);
    keywords.extend(added);
    if let Some(gone) = removed {
        keywords.retain(|key| *key != gone);
    }
    assert!(keywords_of(&after) == keywords, "slot {slot}: the saved keyword list differs from the edits");
    assert!(after == reference_view, "slot {slot}: the saved view differs from the reference");

    let listing = must(
        Registry::new(Language::En).backups(&DataDir::at(saving.data.path()), Game::Ff2Pr),
        &format!("slot {slot}: the backup listing failed"),
    );
    assert!(listing.backups.len() == 1, "slot {slot}: not one backup");
    let backed_up = must(fs::read(listing.backups[0].dir.join(&name)), &format!("slot {slot}: the backup does not read"));
    assert!(backed_up == original, "slot {slot}: the backup differs from the original file");
}

#[test]
fn every_private_ff2_slot_saves_its_edits() {
    let Some(slots) = private_slot_copies(Game::Ff2Pr) else { return };
    let mut counts = Counts::default();
    for (position, private) in slots.into_iter().enumerate() {
        save_and_compare(position + 1, private, &mut counts);
    }
    report(&format!(
        "editor save private ff2: {} slots saved; edits: {} level, {} keyword add, {} gil, {} raw place, {} keyword remove",
        counts.saved, counts.level, counts.keyword_add, counts.plain, counts.raw, counts.keyword_remove
    ));
}
