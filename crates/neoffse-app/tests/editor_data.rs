//! `build` and `EditorViewModel::data`: the Slint data of an open FF I slot.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{FakeLocks, Flow, Part, Site, all_fields, count_of, editing, editing_site, field, fields, lists, pages, row_of, rows, section};
use neoffse::core::{Capability, Entity, FieldId, FieldMeta, FieldView, Game, SaveView, Section, Shape, TableKind, Value};
use neoffse::data::{Language, tables_for};
use neoffse_app::editor::{Input, build};
use neoffse_app::{EditProblem, EditorViewModel, FieldKind};

const NAMES: [&str; 4] = ["Aria", "Bram", "Cleo", "Zoé"];
const SLOTS: [&str; 5] = ["Right hand", "Left hand", "Body", "Head", "Arms"];

fn english_name(kind: TableKind, id: u32) -> String {
    let tables = tables_for(Game::Ff1Pr).unwrap();
    tables.entry(kind, id).unwrap().name(Language::En).to_owned()
}

#[test]
fn ff1_builds_three_present_sections() {
    let site = editing_site();
    let data = editing(&site).data();

    assert!(data.party.present && data.inventory.present && data.magic.present);
    let party = pages(&data.party);
    assert_eq!(party.iter().map(|page| page.label.to_string()).collect::<Vec<_>>(), NAMES);
    assert!(party.iter().all(|page| fields(page).len() == 43));
    assert_eq!(lists(&data.party).len(), 0);

    let inventory = pages(&data.inventory);
    assert_eq!(inventory.len(), 1);
    assert_eq!(fields(&inventory[0]).len(), 1);
    let inventory_lists = lists(&data.inventory);
    assert_eq!(inventory_lists.len(), 2);
    assert_eq!(rows(&inventory_lists[0]).len(), 29);
    assert_eq!(rows(&inventory_lists[1]).len(), 9);
    assert!(inventory_lists.iter().all(|list| list.insertable));

    let magic = pages(&data.magic);
    assert_eq!(magic.iter().map(|page| page.label.to_string()).collect::<Vec<_>>(), NAMES);
    assert!(magic.iter().all(|page| fields(page).len() == 24));
}

#[test]
fn widget_kinds_follow_the_meta() {
    let site = editing_site();
    let data = editing(&site).data();

    for page in pages(&data.party) {
        for row in fields(&page) {
            let label = row.label.to_string();
            let expected = if label == "Name" {
                FieldKind::Text
            } else if label == "Max HP" || label == "Next level" {
                FieldKind::Label
            } else if label == "Job" || SLOTS.contains(&label.as_str()) {
                FieldKind::Picker
            } else {
                FieldKind::Number
            };
            assert_eq!(row.kind, expected, "{label} on a party page");
        }
    }
    for page in pages(&data.magic) {
        assert!(fields(&page).iter().all(|row| row.kind == FieldKind::Label), "a spell slot is not a label");
    }
    for row in all_fields(&data) {
        assert!(
            !matches!(row.kind, FieldKind::Choice | FieldKind::Toggle),
            "{} is a choice or a toggle",
            row.label
        );
    }
    let aria_slot_count = field(&data, Part::Party, "Aria", "Count in slot 1");
    assert_eq!(aria_slot_count.kind, FieldKind::Number);
    assert_eq!(field(&data, Part::Party, "Aria", "Level").kind, FieldKind::Number);
}

#[test]
fn a_picker_row_shows_the_entry_name() {
    let site = editing_site();
    let ed = editing(&site);
    let data = ed.data();

    let job = field(&data, Part::Party, "Aria", "Job");
    assert_eq!(job.text, "7");
    assert_eq!(job.shown.to_string(), english_name(TableKind::Job, 7));
    assert!(job.known);

    let key = field(&data, Part::Party, "Aria", "Right hand").key;
    ed.editor().set_expert(true);
    assert!(ed.editor().commit(key, Input::Pick(999_999)).is_none());
    let data = ed.data();
    let slot = field(&data, Part::Party, "Aria", "Right hand");
    assert!(!slot.known);
    assert_eq!(slot.text, "999999");
    assert_eq!(slot.shown, "");
}

#[test]
fn caps_show_and_mark() {
    let site = editing_site();
    let ed = editing(&site);
    let data = ed.data();
    let level = field(&data, Part::Party, "Aria", "Level");
    assert_eq!(level.cap, "99");
    assert!(!level.above_cap);
    assert_eq!(field(&data, Part::Party, "Aria", "Name").cap, "");

    ed.editor().set_expert(true);
    assert!(ed.editor().commit(level.key, Input::Text("100".into())).is_none());
    let level = field(&ed.data(), Part::Party, "Aria", "Level");
    assert_eq!(level.text, "100", "the number is not clamped to the cap");
    assert!(level.above_cap);
}

#[test]
fn locks_grey_the_default_mode() {
    let site = editing_site();
    let data = editing(&site).data();

    let slot_count = field(&data, Part::Party, "Aria", "Count in slot 1");
    assert!(slot_count.locked && !slot_count.editable);
    let key_item = count_of(&row_of(&data, "Key items", 45));
    assert!(key_item.locked && !key_item.editable);
    let spell = field(&data, Part::Magic, "Aria", "Level 1, slot 1");
    assert!(spell.locked && !spell.editable);
    let gil = field(&data, Part::Inventory, "Gil", "Gil");
    assert!(!gil.locked && gil.editable);

    let worn = row_of(&data, "Items", 89);
    assert!(worn.locked && worn.removable && !worn.can_remove);
    let free = row_of(&data, "Items", 100);
    assert!(!free.locked && free.can_remove);
}

#[test]
fn expert_unlocks_all_but_read_only() {
    let site = editing_site();
    let ed = editing(&site);
    ed.editor().set_expert(true);
    let data = ed.data();

    assert!(field(&data, Part::Party, "Aria", "Count in slot 1").editable);
    assert!(count_of(&row_of(&data, "Key items", 45)).editable);
    let worn = row_of(&data, "Items", 89);
    assert!(worn.locked, "the lock is still reported");
    assert!(worn.can_remove);
    for page in pages(&data.magic) {
        assert!(fields(&page).iter().all(|row| !row.editable));
    }
}

fn party_only() -> SaveView {
    let meta = FieldMeta::new("Level", Shape::Int { min: 0, max: 99 });
    let id = FieldId::new("character").with_id(1).with_name("level");
    let entity = Entity::new(FieldId::new("character").with_id(1), "Hero", vec![FieldView::new(id, meta, Value::Int(5))]);
    SaveView::new(vec![
        Section::new(Capability::Party, vec![entity]),
        Section::new(Capability::Bestiary, Vec::new()),
    ])
}

#[test]
fn a_missing_section_is_absent_and_other_capabilities_are_ignored() {
    let mut locks = FakeLocks::answering(false);
    let built = build(&party_only(), Game::Ff1Pr, Language::En, false, &mut locks);
    assert!(built.data.party.present);
    assert_eq!(pages(&built.data.party).len(), 1);
    assert!(!built.data.inventory.present);
    assert!(!built.data.magic.present);
    assert!(pages(&built.data.inventory).is_empty() && lists(&built.data.inventory).is_empty());
    assert_eq!(locks.asked_fields.len(), 1, "the builder asks about the fields the view holds");
    assert_eq!(locks.asked_rows.len(), 0);
}

#[test]
fn a_failing_lock_query_counts_as_locked() {
    // `Locks` answers a bool: the production impl maps a failed query to `true`.
    let mut locks = FakeLocks::answering(true);
    let built = build(&party_only(), Game::Ff1Pr, Language::En, false, &mut locks);
    let level = &fields(&pages(&built.data.party)[0])[0];
    assert!(level.locked);
    assert!(!level.editable);

    let mut open = FakeLocks::answering(false);
    let built = build(&party_only(), Game::Ff1Pr, Language::En, false, &mut open);
    assert!(fields(&pages(&built.data.party)[0])[0].editable);
}

#[test]
fn a_wrong_kind_value_shows_as_a_label() {
    let meta = FieldMeta::new("Name", Shape::Text);
    let id = FieldId::new("character").with_id(1).with_name("name");
    let entity = Entity::new(FieldId::new("character").with_id(1), "Hero", vec![FieldView::new(id, meta, Value::Int(5))]);
    let view = SaveView::new(vec![Section::new(Capability::Party, vec![entity])]);
    let built = build(&view, Game::Ff1Pr, Language::En, true, &mut FakeLocks::answering(false));

    let row = &fields(&pages(&built.data.party)[0])[0];
    assert_eq!(row.kind, FieldKind::Label);
    assert_eq!(row.text, "");
    assert_eq!(row.shown, "");
    assert!(!row.editable);
}

#[test]
fn keys_are_dense_and_unique() {
    let site = editing_site();
    let ed = editing(&site);
    for expert in [false, true] {
        ed.editor().set_expert(expert);
        let data = ed.data();
        let keys: Vec<i32> = all_fields(&data).iter().map(|row| row.key).collect();
        assert_eq!(keys, (0..i32::try_from(keys.len()).unwrap()).collect::<Vec<_>>());

        let all_lists: Vec<_> = [Part::Party, Part::Inventory, Part::Magic]
            .into_iter()
            .flat_map(|part| lists(&section(&data, part)))
            .collect();
        let list_keys: Vec<i32> = all_lists.iter().map(|list| list.key).collect();
        assert_eq!(list_keys, vec![0, 1]);
        let row_keys: Vec<i32> = all_lists.iter().flat_map(rows).map(|row| row.key).collect();
        assert_eq!(row_keys, (0..38).collect::<Vec<_>>());
    }
}

#[test]
fn data_without_a_save_is_an_internal_notice() {
    let site = Site::new();
    let flow = Flow::new(&site, None);
    let notice = flow.editor.data().expect_err("no save is open");
    assert_eq!(notice.problem, EditProblem::Internal);
    assert_eq!(notice.label, "");
}

#[test]
fn the_model_is_send_and_sync() {
    fn needs<T: Send + Sync>() {}
    needs::<EditorViewModel>();
}

#[test]
fn debug_prints_flags_only() {
    let site = editing_site();
    let ed = editing(&site);
    let printed = format!("{:?}", ed.editor());
    assert!(printed.contains("open: true") && printed.contains("expert: false"));
    assert!(!printed.contains("Aria"));
}

#[test]
fn a_section_of_read_only_fields_says_so() {
    let site = editing_site();
    let ed = editing(&site);
    for expert in [false, true] {
        ed.editor().set_expert(expert);
        let data = ed.data();
        assert!(data.magic.read_only, "Magic holds only read-only fields");
        assert!(!data.party.read_only);
        assert!(!data.inventory.read_only);
    }
    let mut locks = FakeLocks::answering(false);
    let built = build(&party_only(), Game::Ff1Pr, Language::En, false, &mut locks);
    assert!(!built.data.magic.read_only, "an absent section is not read only");
}
