//! `EditorViewModel::add_row`, `remove_row` and the picker entries.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::sync::Arc;

use common::{Editing, Part, count_of, editing, editing_site, field, item_label, list, row_of, rows};
use neoffse::core::{Game, TableKind};
use neoffse::data::{Language, tables_for};
use neoffse_app::{EditProblem, EditorData, EditorViewModel};

fn labels(data: &EditorData, title: &str) -> Vec<String> {
    rows(&list(data, title)).iter().map(|row| row.label.to_string()).collect()
}

fn content_name(id: u32) -> String {
    tables_for(Game::Ff1Pr).unwrap().content(id).unwrap().name(Language::En).to_owned()
}

fn right_hand(ed: &Editing) -> String {
    field(&ed.data(), Part::Party, "Aria", "Right hand").text.to_string()
}

#[test]
fn adding_a_key_item_inserts_a_row_with_count_one() {
    let site = editing_site();
    let ed = editing(&site);
    let key_items = list(&ed.data(), "Key items").key;

    assert!(ed.editor().add_row(key_items, 47).is_none());
    assert_eq!(ed.pending(), 1);
    let data = ed.data();
    assert_eq!(rows(&list(&data, "Key items")).len(), 10);
    assert_eq!(count_of(&row_of(&data, "Key items", 47)).text, "1");
}

#[test]
fn a_duplicate_row_is_refused() {
    let site = editing_site();
    let ed = editing(&site);
    let items = list(&ed.data(), "Items").key;

    let notice = ed.editor().add_row(items, 100).expect("already held");
    assert_eq!(notice.problem, EditProblem::DuplicateRow);
    assert_eq!(notice.label.to_string(), content_name(100));
    assert_eq!(ed.pending(), 0);
}

#[test]
fn an_unknown_or_negative_id_is_refused() {
    let site = editing_site();
    let ed = editing(&site);
    let items = list(&ed.data(), "Items").key;

    let unknown = ed.editor().add_row(items, 99_999).expect("no such entry");
    assert_eq!(unknown.problem, EditProblem::UnknownId);
    assert_eq!(unknown.value, "99999");
    assert_eq!(ed.editor().add_row(items, -5).expect("negative").problem, EditProblem::NotEditable);
    assert_eq!(ed.editor().add_row(99, 100).expect("no such list").problem, EditProblem::NotEditable);
    assert_eq!(ed.pending(), 0);
}

#[test]
fn removing_a_free_row_removes_it_and_undo_restores_it() {
    let site = editing_site();
    let ed = editing(&site);
    let before = ed.data();
    let order = labels(&before, "Items");

    assert!(ed.editor().remove_row(row_of(&before, "Items", 100).key).is_none());
    assert_eq!(ed.pending(), 1);
    assert_eq!(labels(&ed.data(), "Items").len(), 28);

    ed.undo();
    assert_eq!(labels(&ed.data(), "Items"), order);
}

#[test]
fn a_worn_row_cannot_be_removed_in_the_default_mode() {
    let site = editing_site();
    let ed = editing(&site);
    let key = row_of(&ed.data(), "Items", 89).key;
    let slot = right_hand(&ed);

    let notice = ed.editor().remove_row(key).expect("worn");
    assert_eq!(notice.problem, EditProblem::ReadOnly);
    assert_eq!(notice.label.to_string(), item_label(89, "Item"));
    assert_eq!(ed.pending(), 0);

    ed.editor().set_expert(true);
    assert!(ed.editor().remove_row(key).is_none());
    assert_eq!(ed.pending(), 1);
    assert_eq!(labels(&ed.data(), "Items").len(), 28);
    assert_eq!(right_hand(&ed), slot, "the equipment slot is untouched");
}

#[test]
fn a_placeholder_row_cannot_be_removed_by_default() {
    let site = editing_site();
    let ed = editing(&site);
    let row = row_of(&ed.data(), "Items", 62);
    assert!(row.locked && !row.can_remove);
    let notice = ed.editor().remove_row(row.key).expect("placeholder");
    assert_eq!(notice.problem, EditProblem::ReadOnly);
    assert_eq!(ed.pending(), 0);
}

#[test]
fn placeholders_are_not_offered_for_adding() {
    let site = editing_site();
    let ed = editing(&site);
    let data = ed.data();
    let tables = tables_for(Game::Ff1Pr).unwrap();
    let placeholders: Vec<i32> = [TableKind::Weapon, TableKind::Shield, TableKind::Armor, TableKind::Helmet, TableKind::Gloves]
        .into_iter()
        .map(|kind| i32::try_from(tables.placeholder(kind).unwrap().id).unwrap())
        .collect();

    let offered = ed.editor().list_entries(list(&data, "Items").key, "");
    assert_ne!(offered.len(), 0);
    assert!(offered.iter().all(|entry| !placeholders.contains(&entry.id)));

    let slot = field(&data, Part::Party, "Aria", "Right hand").key;
    let weapon_placeholder = i32::try_from(tables.placeholder(TableKind::Weapon).unwrap().id).unwrap();
    assert!(ed.editor().field_entries(slot, "").iter().any(|entry| entry.id == weapon_placeholder));

    assert_eq!(ed.editor().field_entries(9999, "").len(), 0);
    assert_eq!(ed.editor().list_entries(99, "").len(), 0);
}

#[test]
fn entries_follow_the_language() {
    let site = editing_site();
    let ed = editing(&site);
    let tables = tables_for(Game::Ff1Pr).unwrap();
    let differing = tables
        .table(TableKind::Item)
        .unwrap()
        .iter()
        .find(|entry| !entry.name_en.is_empty() && !entry.name_fr.is_empty() && entry.name_en != entry.name_fr)
        .expect("an item whose names differ");

    let items = list(&ed.data(), "Items").key;
    let name_of = |editor: &EditorViewModel| {
        editor
            .list_entries(items, "")
            .into_iter()
            .find(|entry| i64::from(entry.id) == i64::from(differing.id))
            .expect("the entry")
            .name
            .to_string()
    };
    assert_eq!(name_of(ed.editor()), differing.name_en);

    let french = EditorViewModel::new(Arc::clone(&ed.flow.state), Language::Fr);
    french.data().unwrap();
    assert_eq!(name_of(&french), differing.name_fr);
}

#[test]
fn a_row_edit_changes_the_keys_after_data() {
    let site = editing_site();
    let ed = editing(&site);
    let before = ed.data();
    let last = i32::try_from(labels(&before, "Items").len() + labels(&before, "Key items").len()).unwrap() - 1;

    assert!(ed.editor().remove_row(row_of(&before, "Items", 100).key).is_none());
    let after = ed.data();
    assert_eq!(labels(&after, "Items").len(), 28);
    let row_keys: Vec<i32> = ["Items", "Key items"]
        .iter()
        .flat_map(|title| rows(&list(&after, title)))
        .map(|row| row.key)
        .collect();
    assert_eq!(row_keys, (0..last).collect::<Vec<_>>());
    assert_eq!(ed.editor().remove_row(last).expect("the old last key").problem, EditProblem::NotEditable);
}
