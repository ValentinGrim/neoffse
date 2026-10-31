//! The pages only expert mode edits: hidden in the default mode, shown in
//! expert mode, and the review that follows the same rule. The FF II Magic
//! section holds a Level page and a raw page per character; FF I's Magic pages
//! stay in both modes. No window shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{
    Editing, FakeLocks, ID_1, Part, editing, editing_site, ff2_editing, ff2_site, field, fields, lists, page, pages, saving_over, section, slot_file,
    steam_folder,
};
use neoffse::Running;
use neoffse::core::{Capability, Entity, FieldId, FieldMeta, FieldView, Game, SaveView, Section, Shape, Value};
use neoffse::data::Language;
use neoffse_app::editor::{Input, build};
use neoffse_app::{ReviewKind, ReviewRow};

const NAMES: [&str; 4] = ["Ivo", "Mara", "Tane", "Élan"];
/// Places per character of the fixture: three per list entry, three per
/// dictionary entry, then the two order lists.
const PLACES: [usize; 4] = [28, 36, 12, 132];

fn labels(ed: &Editing, part: Part) -> Vec<String> {
    pages(&section(&ed.data(), part)).iter().map(|page| page.label.to_string()).collect()
}

/// The entity label of a raw page, which the review headings print. The
/// button of the page reads the name alone.
fn raw_label(name: &str) -> String {
    format!("{name}, raw spells")
}

fn kinds(rows: &[ReviewRow]) -> Vec<ReviewKind> {
    rows.iter().map(|row| row.kind).collect()
}

fn field_rows(rows: &[ReviewRow]) -> usize {
    rows.iter().filter(|row| row.kind == ReviewKind::Field).count()
}

#[test]
fn default_mode_has_no_raw_page() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    assert_eq!(labels(&ed, Part::Magic), NAMES);
    let data = ed.data();
    assert!(!data.raw_magic.present);
    assert_eq!(pages(&data.raw_magic).len(), 0);
}

#[test]
fn expert_mode_moves_the_raw_pages_to_their_own_section() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    ed.editor().set_expert(true);
    assert_eq!(labels(&ed, Part::Magic), NAMES);
    assert_eq!(labels(&ed, Part::RawMagic), NAMES);
    let data = ed.data();
    assert!(data.raw_magic.present);
    assert!(!data.raw_magic.read_only);
    assert_eq!(lists(&data.raw_magic).len(), 0);
    for (name, places) in NAMES.iter().zip(PLACES) {
        let rows = fields(&page(&data, Part::RawMagic, name));
        assert_eq!(rows.len(), places, "{name}");
        assert!(
            rows.iter().all(|row| row.editable && row.locked),
            "{name}: a place is not editable in expert mode"
        );
    }
}

#[test]
fn the_raw_buttons_read_the_character_names() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    ed.editor().set_expert(true);
    let found = labels(&ed, Part::RawMagic);
    assert_eq!(found, NAMES);
    assert!(found.iter().all(|label| !label.contains("raw")));
}

#[test]
fn the_711_entry_is_a_listed_picker_without_a_name() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    ed.editor().set_expert(true);
    let row = field(&ed.data(), Part::RawMagic, "Ivo", "List 1, Entry 711, row");
    assert_eq!(row.kind, neoffse_app::FieldKind::Picker);
    assert_eq!(row.text, "711");
    assert!(row.known, "the table lists row 711");
    // The row has no name: the picker draws `Entry 711` from the id (`fields.slint`).
    assert_eq!(row.shown, "");
}

#[test]
fn a_one_sided_proficiency_commit_then_undo_restores_every_byte() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    ed.editor().set_expert(true);
    let before = ed.with_session(|session| session.view().unwrap());
    let place = field(&ed.data(), Part::RawMagic, "Ivo", "List 2, Fire, value");
    assert_ne!(place.text, "456");

    assert!(ed.editor().commit(place.key, Input::Text("456".to_owned())).is_none());
    let place = field(&ed.data(), Part::RawMagic, "Ivo", "List 2, Fire, value");
    assert_eq!(place.text, "456");
    assert_eq!(field(&ed.data(), Part::Magic, "Ivo", "Fire").text, "5", "the Level field still reads the row");
    assert_eq!(ed.pending(), 1);

    assert!(ed.editor().undo().is_none());
    let after = ed.with_session(|session| session.view().unwrap());
    assert!(after == before, "the view differs after the undo");
    let changes = ed.with_session(|session| session.changes(Language::En).unwrap());
    assert_eq!(changes.len(), 0);
}

#[test]
fn ff1_magic_pages_show_in_default_mode() {
    let site = editing_site();
    let ed = editing(&site);
    let data = ed.data();
    assert_eq!(pages(&section(&data, Part::Magic)).len(), 4);
    assert!(data.magic.read_only);
    assert!(!data.raw_magic.present);
    ed.editor().set_expert(true);
    let data = ed.data();
    assert_eq!(pages(&data.magic).len(), 4);
    assert!(!data.raw_magic.present);
}

fn ints(page: &str, read_only: bool, count: u32) -> Entity {
    let id = FieldId::new("page");
    let fields = (0..count)
        .map(|index| {
            let mut meta = FieldMeta::new("Value", Shape::Int { min: 0, max: 9 });
            meta.read_only = read_only;
            FieldView::new(id.clone().with_id(index), meta, Value::Int(0))
        })
        .collect();
    Entity::new(id, page, fields)
}

fn magic_view(entities: Vec<Entity>) -> SaveView {
    SaveView::new(vec![Section::new(Capability::Magic, entities)])
}

/// The labels of the Magic pages and of the Raw spells pages.
type LabelPair = (Vec<String>, Vec<String>);

fn page_labels(view: &SaveView, expert: bool, locks: &mut dyn neoffse_app::editor::Locks) -> LabelPair {
    let built = build(view, Game::Ff2Pr, Language::En, expert, locks);
    let of = |section: &neoffse_app::SectionData| pages(section).iter().map(|page| page.label.to_string()).collect();
    (of(&built.data.magic), of(&built.data.raw_magic))
}

fn strings(labels: &[&str]) -> Vec<String> {
    labels.iter().map(ToString::to_string).collect()
}

#[test]
fn a_page_of_locked_editable_fields_is_expert_only() {
    let view = magic_view(vec![ints("Raw", false, 2)]);
    assert_eq!(page_labels(&view, false, &mut FakeLocks::answering(true)), (vec![], vec![]));
    assert_eq!(page_labels(&view, true, &mut FakeLocks::answering(true)), (vec![], strings(&["Raw"])));
    assert_eq!(page_labels(&view, false, &mut FakeLocks::answering(false)), (strings(&["Raw"]), vec![]));
}

#[test]
fn a_read_only_page_and_an_empty_page_stay() {
    let view = magic_view(vec![ints("Reads", true, 2), ints("Empty", false, 0)]);
    assert_eq!(
        page_labels(&view, false, &mut FakeLocks::answering(true)),
        (strings(&["Reads", "Empty"]), vec![])
    );
}

/// Locks the fields whose id starts with one of the prefixes.
struct Under(Vec<FieldId>);
impl neoffse_app::editor::Locks for Under {
    fn field_locked(&mut self, field: &FieldId) -> bool {
        self.0.iter().any(|prefix| field.starts_with(prefix))
    }

    fn row_locked(&mut self, _: &neoffse::core::RowId) -> bool {
        false
    }
}

fn sheet(id: FieldId, label: &str) -> Entity {
    let meta = FieldMeta::new("Value", Shape::Int { min: 0, max: 9 });
    Entity::new(id.clone(), label, vec![FieldView::new(id.with_id(0), meta, Value::Int(0))])
}

#[test]
fn an_expert_page_without_a_parent_page_keeps_its_label() {
    let hero = FieldId::new("sheet").with_id(1);
    let loose = FieldId::new("sheet").with_id(7);
    let view = magic_view(vec![
        sheet(hero.clone(), "Hero"),
        sheet(hero.clone().with_name("raw"), "Hero, raw"),
        sheet(loose.clone().with_name("raw"), "Loose"),
    ]);
    let mut locks = Under(vec![hero.with_name("raw"), loose.with_name("raw")]);
    let (magic, raw) = page_labels(&view, true, &mut locks);
    assert_eq!(magic, ["Hero"]);
    assert_eq!(raw, ["Hero", "Loose"]);
}

/// Locks one field and every row.
struct First(FieldId);
impl neoffse_app::editor::Locks for First {
    fn field_locked(&mut self, field: &FieldId) -> bool {
        *field == self.0
    }

    fn row_locked(&mut self, _: &neoffse::core::RowId) -> bool {
        true
    }
}

#[test]
fn one_unlocked_field_keeps_the_page() {
    let id = FieldId::new("page");
    let meta = || FieldMeta::new("Value", Shape::Int { min: 0, max: 9 });
    let mixed = Entity::new(
        id.clone(),
        "Mixed",
        vec![
            FieldView::new(id.clone().with_id(0), meta(), Value::Int(0)),
            FieldView::new(id.clone().with_id(1), meta(), Value::Int(0)),
        ],
    );
    let view = magic_view(vec![mixed]);
    let built = build(&view, Game::Ff2Pr, Language::En, false, &mut First(id.with_id(0)));
    assert_eq!(pages(&built.data.magic).len(), 1);
}

fn level_edit(ed: &Editing) {
    let key = field(&ed.data(), Part::Magic, "Ivo", "Fire").key;
    assert!(ed.editor().commit(key, Input::Text("9".to_owned())).is_none());
}

fn proficiency_edit(ed: &Editing) {
    let was = ed.editor().expert();
    ed.editor().set_expert(true);
    let key = field(&ed.data(), Part::RawMagic, "Ivo", "List 2, Fire, value").key;
    assert!(ed.editor().commit(key, Input::Text("456".to_owned())).is_none());
    ed.editor().set_expert(was);
}

#[test]
fn in_default_mode_a_level_edit_gives_one_field_row() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    level_edit(&ed);
    let rows = ed.editor().review().unwrap();
    assert_eq!(kinds(&rows), [ReviewKind::Heading, ReviewKind::Field]);
    assert_eq!((rows[1].label.as_str(), rows[1].before.as_str(), rows[1].after.as_str()), ("Fire", "5", "9"));
}

#[test]
fn in_expert_mode_the_same_edit_gives_nine_field_rows() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    level_edit(&ed);
    ed.editor().set_expert(true);
    let rows = ed.editor().review().unwrap();
    assert_eq!(field_rows(&rows), 9);
    assert_eq!(rows.iter().filter(|row| row.kind == ReviewKind::Heading).count(), 2);
    ed.editor().set_expert(false);
    assert_eq!(field_rows(&ed.editor().review().unwrap()), 1);
}

#[test]
fn a_raw_edit_stays_in_the_review_after_expert_mode_ends() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    proficiency_edit(&ed);
    assert!(!ed.editor().expert());
    let rows = ed.editor().review().unwrap();
    assert_eq!(field_rows(&rows), 1);
    assert_eq!(rows[0].entity, raw_label("Ivo"));
    // A Level edit now lists its raw rows too: the flag holds until the session is clean.
    // The Level row and eight places make nine; the earlier edit shares one of the places.
    level_edit(&ed);
    assert_eq!(field_rows(&ed.editor().review().unwrap()), 9);
}

#[test]
fn a_save_clears_the_flag() {
    let site = ff2_site();
    let file = steam_folder(&site, 1, ID_1).join(slot_file(1));
    let saving = saving_over(ff2_editing(&site), file);
    proficiency_edit(&saving.ed);
    saving.save(Running::Refuse);
    saving.ed.flow.manual.run_all();
    assert_eq!(saving.outcomes().len(), 1);
    assert_eq!(saving.ed.pending(), 0);

    level_edit(&saving.ed);
    let rows = saving.ed.editor().review().unwrap();
    assert_eq!(kinds(&rows), [ReviewKind::Heading, ReviewKind::Field]);
}

#[test]
fn undo_past_the_clean_point_shows_every_row() {
    let site = ff2_site();
    let file = steam_folder(&site, 1, ID_1).join(slot_file(1));
    let saving = saving_over(ff2_editing(&site), file);
    proficiency_edit(&saving.ed);
    saving.save(Running::Refuse);
    saving.ed.flow.manual.run_all();
    assert_eq!(saving.ed.pending(), 0);
    assert!(!saving.ed.editor().expert());

    assert!(saving.ed.editor().undo().is_none());
    assert_eq!(saving.ed.pending(), 1);
    let rows = saving.ed.editor().review().unwrap();
    assert_eq!(field_rows(&rows), 1, "the reverse of the saved raw edit is listed");
    assert_eq!(rows[0].entity, raw_label("Ivo"));
}

#[test]
fn undo_to_the_clean_point_clears_the_flag() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    proficiency_edit(&ed);
    assert!(ed.editor().undo().is_none());
    assert_eq!(ed.pending(), 0);
    level_edit(&ed);
    assert_eq!(field_rows(&ed.editor().review().unwrap()), 1, "the flag is gone");
}

#[test]
fn redo_of_a_raw_edit_from_the_clean_point_shows_it() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    proficiency_edit(&ed);
    assert!(ed.editor().undo().is_none());
    assert!(ed.editor().redo().is_none());
    assert_eq!(ed.pending(), 1);
    assert_eq!(field_rows(&ed.editor().review().unwrap()), 1);
}

#[test]
fn a_new_file_starts_without_the_flag() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    proficiency_edit(&ed);
    ed.editor().back();
    let reopened = ff2_editing(&site);
    level_edit(&reopened);
    assert_eq!(field_rows(&reopened.editor().review().unwrap()), 1);
}

#[test]
fn the_review_keeps_the_capability_and_the_headings_of_a_raw_edit() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    level_edit(&ed);
    ed.editor().set_expert(true);
    let changes = ed.with_session(|session| session.changes(Language::En).unwrap());
    assert_eq!(changes.len(), 9);
    assert!(changes.iter().all(|change| change.capability == Capability::Magic));
    assert_eq!(changes.iter().filter(|change| change.entity_label == "Ivo").count(), 1);
    assert_eq!(changes.iter().filter(|change| change.entity_label == raw_label("Ivo")).count(), 8);
}
