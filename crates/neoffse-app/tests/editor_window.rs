//! The editor page wired to its view model through the `EditorBridge`
//! global, on Slint's testing backend. No window shows and no event loop
//! runs. The tests drive the bridge callbacks the screens call.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Part, Ui, count_of, editing_site, field, item_label, list, lists, open_editor, pages, row_of, rows};
use neoffse::core::{Game, TableKind};
use neoffse::data::tables_for;
use neoffse_app::{EditProblem, EditorBridge, EditorData, ItemList};
use slint::{ComponentHandle, Model};

#[test]
fn opening_a_save_fills_the_bridge() {
    let site = editing_site();
    let ui = open_editor(&site);
    let data = ui.bridge_data();

    for section in [&data.party, &data.inventory, &data.magic] {
        assert!(section.present);
    }
    let party = pages(&data.party);
    assert_eq!(party.len(), 4);
    assert!(party.iter().all(|page| page.fields.row_count() == 41));
    let inventory: Vec<usize> = lists(&data.inventory).iter().map(|list| list.rows.row_count()).collect();
    assert_eq!(inventory, [29, 9]);
    assert!(!ui.window.global::<EditorBridge>().get_has_notice());
}

#[test]
fn a_text_commit_edits_and_refreshes() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.gil().key, "500");

    assert_eq!(ui.gil().text, "500");
    assert_eq!(ui.pending(), 1);
    assert!(!ui.window.global::<EditorBridge>().get_has_notice());
}

#[test]
fn a_refused_commit_shows_a_notice() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.window.global::<EditorBridge>();
    let before = ui.level();
    ui.commit(before.key, "100");

    assert!(bridge.get_has_notice());
    let notice = bridge.get_notice();
    assert_eq!(notice.problem, EditProblem::AboveCap);
    assert_eq!(notice.value, "100");
    assert_eq!(notice.cap, "99");
    assert_eq!(notice.label, "Level");
    assert_eq!(ui.level().text, before.text, "the row still reads the document's value");
    assert_eq!(ui.pending(), 0);

    bridge.invoke_notice_dismissed();
    assert!(!bridge.get_has_notice());

    ui.commit(before.key, "100");
    assert!(bridge.get_has_notice());
    ui.commit(ui.gil().key, "500");
    assert!(!bridge.get_has_notice(), "a successful commit clears the notice");
}

#[test]
fn bad_text_shows_not_a_number() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.gil().key, "abc");
    let notice = ui.window.global::<EditorBridge>().get_notice();
    assert!(ui.window.global::<EditorBridge>().get_has_notice());
    assert_eq!(notice.problem, EditProblem::NotANumber);
    assert_eq!(notice.label, "Gil");
    assert_eq!(ui.pending(), 0);
}

/// A non-placeholder weapon id the inventory does not hold.
fn spare_weapon(data: &EditorData) -> u32 {
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

#[test]
fn a_pick_equips_in_one_entry() {
    let site = editing_site();
    let ui = open_editor(&site);
    let before = ui.bridge_data();
    let id = spare_weapon(&before);
    let slot = field(&before, Part::Party, "Aria", "Right hand");
    let rows_before = rows(&list(&before, "Items")).len();

    ui.window.global::<EditorBridge>().invoke_field_picked(slot.key, i32::try_from(id).unwrap());

    let after = ui.bridge_data();
    assert_eq!(rows(&list(&after, "Items")).len(), rows_before + 1, "the same refresh shows the new row");
    assert_eq!(count_of(&row_of(&after, "Items", id)).text, "1");
    assert_eq!(field(&after, Part::Party, "Aria", "Right hand").text.to_string(), id.to_string());
    assert_eq!(ui.pending(), 1);
    assert!(!ui.window.global::<EditorBridge>().get_has_notice());
}

#[test]
fn a_choice_and_a_toggle_route_to_the_view_model() {
    // No FF I field is a choice or a toggle: the calls reach the view model
    // on a number field and come back as a refusal.
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.window.global::<EditorBridge>();
    let key = ui.gil().key;

    bridge.invoke_field_toggled(key, true);
    assert_eq!(bridge.get_notice().problem, EditProblem::NotEditable);
    bridge.invoke_notice_dismissed();

    bridge.invoke_field_name_picked(key, 0);
    assert!(bridge.get_has_notice());
    assert_eq!(bridge.get_notice().problem, EditProblem::NotEditable);
    bridge.invoke_notice_dismissed();

    bridge.invoke_field_name_picked(key, -1);
    assert!(bridge.get_has_notice());
    assert_eq!(bridge.get_notice().problem, EditProblem::NotEditable);
    assert_eq!(ui.pending(), 0);
}

fn entry_ids(ui: &Ui) -> Vec<i32> {
    ui.window.global::<EditorBridge>().get_picker_entries().iter().map(|entry| entry.id).collect()
}

#[test]
fn picker_queries_fill_the_entries() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.window.global::<EditorBridge>();
    let tables = tables_for(Game::Ff1Pr).unwrap();
    let jobs = tables.table(TableKind::Job).unwrap();
    let job = field(&ui.bridge_data(), Part::Party, "Aria", "Job").key;

    bridge.invoke_field_picker_query(job, "".into());
    assert_eq!(bridge.get_picker_entries().row_count(), jobs.len());

    let first = jobs.iter().find(|entry| !entry.name_en.is_empty()).expect("a named job");
    bridge.invoke_field_picker_query(job, first.name_en.to_lowercase().as_str().into());
    let narrowed = entry_ids(&ui);
    assert!(narrowed.contains(&i32::try_from(first.id).unwrap()));
    assert!(narrowed.len() < jobs.len());

    bridge.invoke_field_picker_query(9999, "".into());
    assert_eq!(bridge.get_picker_entries().row_count(), 0);

    let items: ItemList = list(&ui.bridge_data(), "Items");
    bridge.invoke_list_picker_query(items.key, "".into());
    let ids = entry_ids(&ui);
    assert_ne!(ids.len(), 0);
    for kind in [TableKind::Weapon, TableKind::Shield, TableKind::Armor, TableKind::Helmet, TableKind::Gloves] {
        let placeholder = i32::try_from(tables.placeholder(kind).unwrap().id).unwrap();
        assert!(!ids.contains(&placeholder), "a placeholder is offered for adding");
    }
}

#[test]
fn no_window_is_ever_shown() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.window.global::<EditorBridge>();
    ui.commit(ui.gil().key, "500");
    ui.commit(ui.level().key, "100");
    bridge.invoke_field_picker_query(field(&ui.bridge_data(), Part::Party, "Aria", "Job").key, "".into());
    bridge.invoke_notice_dismissed();
    ui.window.invoke_back_requested();
    ui.commit(0, "1");
    assert!(!ui.window.window().is_visible());
}
