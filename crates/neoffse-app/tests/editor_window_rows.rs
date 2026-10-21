//! Rows, in-place refresh and the end of a session on the editor page, through
//! the `EditorBridge` global on Slint's testing backend. No window shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Part, all_fields, editing_site, field, item_label, list, open_editor, pages, row_of, rows};
use neoffse_app::{EditProblem, EditorBridge};
use slint::{ComponentHandle, Model};

#[test]
fn add_and_remove_rows() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.window.global::<EditorBridge>();

    let key_items = list(&ui.bridge_data(), "Key items").key;
    bridge.invoke_item_add_requested(key_items, 47);
    assert_eq!(rows(&list(&ui.bridge_data(), "Key items")).len(), 10);
    assert_eq!(ui.pending(), 1);
    assert!(!bridge.get_has_notice());

    let free = row_of(&ui.bridge_data(), "Items", 100).key;
    bridge.invoke_item_remove_requested(free);
    assert_eq!(rows(&list(&ui.bridge_data(), "Items")).len(), 28);
    assert_eq!(ui.pending(), 2);

    let worn = row_of(&ui.bridge_data(), "Items", 89).key;
    bridge.invoke_item_remove_requested(worn);
    assert!(bridge.get_has_notice());
    assert_eq!(bridge.get_notice().problem, EditProblem::ReadOnly);
    assert_eq!(bridge.get_notice().label, item_label(89, "Item"));
    assert_eq!(ui.pending(), 2);
    assert_eq!(rows(&list(&ui.bridge_data(), "Items")).len(), 28);
}

#[test]
fn the_refresh_updates_in_place() {
    let site = editing_site();
    let ui = open_editor(&site);
    let held = pages(&ui.bridge_data().party)[0].fields.clone();
    let gil_page = pages(&ui.bridge_data().inventory)[0].fields.clone();

    ui.commit(ui.gil().key, "500");
    ui.commit(ui.level().key, "50");

    let after = ui.bridge_data();
    assert_eq!(pages(&after.party)[0].fields, held, "the fields model is the one the screen already shows");
    assert_eq!(pages(&after.inventory)[0].fields, gil_page);
    assert_eq!(ui.gil().text, "500");
    assert_eq!(ui.level().text, "50");
    assert_eq!(held.iter().find(|row| row.label == "Level").unwrap().text, "50", "the held model changed");

    // A new page label keeps the fields model too.
    ui.commit(field(&after, Part::Party, "Aria", "Name").key, "Zed");
    let renamed = pages(&ui.bridge_data().party);
    assert_eq!(renamed[0].label, "Zed");
    assert_eq!(renamed[0].fields, held);

    // A removal changes the shape: the rows may come as new models.
    let free = row_of(&ui.bridge_data(), "Items", 100).key;
    ui.window.global::<EditorBridge>().invoke_item_remove_requested(free);
    assert_eq!(rows(&list(&ui.bridge_data(), "Items")).len(), 28);
}

#[test]
fn the_refresh_after_an_edit_keeps_the_keys_dense() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.window.global::<EditorBridge>();
    ui.commit(ui.gil().key, "500");
    bridge.invoke_item_remove_requested(row_of(&ui.bridge_data(), "Items", 100).key);
    bridge.invoke_item_add_requested(list(&ui.bridge_data(), "Key items").key, 47);

    let data = ui.bridge_data();
    let keys: Vec<i32> = all_fields(&data).iter().map(|row| row.key).collect();
    assert_eq!(keys, (0..i32::try_from(keys.len()).unwrap()).collect::<Vec<_>>());
    let row_keys: Vec<i32> = ["Items", "Key items"]
        .iter()
        .flat_map(|title| rows(&list(&data, title)))
        .map(|row| row.key)
        .collect();
    assert_eq!(row_keys, (0..i32::try_from(row_keys.len()).unwrap()).collect::<Vec<_>>());
}

#[test]
fn editing_after_back_is_harmless() {
    let site = editing_site();
    let ui = open_editor(&site);
    let bridge = ui.window.global::<EditorBridge>();
    let key = ui.gil().key;

    ui.window.invoke_back_requested();
    assert!(!ui.window.get_editor_open());
    ui.commit(key, "500");
    assert!(bridge.get_has_notice());
    assert_eq!(bridge.get_notice().problem, EditProblem::NotEditable);
    bridge.invoke_item_remove_requested(0);
    bridge.invoke_item_add_requested(0, 47);
    bridge.invoke_field_picker_query(key, "".into());
    assert_eq!(bridge.get_notice().problem, EditProblem::NotEditable);
    assert!(!ui.app.state.is_open());
}

#[test]
fn a_second_save_opens_with_a_clean_notice() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.level().key, "100");
    assert!(ui.window.global::<EditorBridge>().get_has_notice());

    ui.window.invoke_back_requested();
    let key = ui.first_slots()[0].key;
    ui.window.invoke_slot_clicked(key);
    ui.manual.run_all();
    assert!(ui.window.get_editor_open());
    assert!(!ui.window.global::<EditorBridge>().get_has_notice(), "a new save starts without a notice");
    assert_eq!(ui.gil().text, "123456");
}
