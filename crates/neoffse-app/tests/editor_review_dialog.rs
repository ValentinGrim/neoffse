//! The review dialog and Ctrl+S: what opens it, what it lists, how it
//! closes. The testing backend draws nothing, so a test reads the bridge and
//! the accessibility tree.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{ID_1, Ui, editing_site, items, list, open_editor, row_of, slot_file, steam_folder};
use i_slint_backend_testing::AccessibleRole::Button;
use neoffse::io::dirs::DataDir;
use neoffse_app::{EditorCommand, ReviewKind, ReviewRow};
use slint::platform::Key;

fn rows(ui: &Ui) -> Vec<ReviewRow> {
    items(&ui.bridge().get_review_rows())
}

fn row(kind: ReviewKind, entity: &str, label: &str, before: &str, after: &str) -> ReviewRow {
    ReviewRow {
        kind,
        entity: entity.into(),
        label: label.into(),
        before: before.into(),
        after: after.into(),
    }
}

/// A dirty save (gil "500") with the review opened by Ctrl+S.
fn reviewing(ui: &Ui) {
    ui.make_dirty();
    ui.chord(&[Key::Control], "s");
    assert!(ui.bridge().get_review_open());
}

#[test]
fn ctrl_s_opens_the_review_with_the_net_difference() {
    let site = editing_site();
    let ui = open_editor(&site);
    reviewing(&ui);

    assert!(ui.bridge().get_blocked());
    assert_eq!(
        rows(&ui),
        vec![
            row(ReviewKind::Heading, "Gil", "", "", ""),
            row(ReviewKind::Field, "Gil", "Gil", "123456", "500"),
        ]
    );
}

#[test]
fn the_bar_button_opens_the_same_review() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.element("Review & save", Button).invoke_accessible_default_action();

    assert!(ui.bridge().get_review_open());
    assert_eq!(rows(&ui).len(), 2);
}

#[test]
fn ctrl_s_never_saves() {
    let site = editing_site();
    let ui = open_editor(&site);
    let data = tempfile::tempdir().unwrap();
    ui.app.state.set_data_dir(DataDir::at(data.path()));
    let file = steam_folder(&site, 0, ID_1).join(slot_file(1));
    let before = fs::read(&file).unwrap();

    reviewing(&ui);

    assert_eq!(fs::read(&file).unwrap(), before);
    assert_eq!(fs::read_dir(data.path()).unwrap().count(), 0);
    assert_eq!(ui.pending(), 1);
}

#[test]
fn ctrl_s_on_a_clean_save_opens_nothing() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.chord(&[Key::Control], "s");
    assert!(!ui.bridge().get_review_open());
    assert!(!ui.bridge().get_blocked());
}

#[test]
fn a_set_back_value_opens_an_empty_review() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.gil().key, "500");
    ui.commit(ui.gil().key, "123456");
    assert_eq!(ui.pending(), 2);

    ui.chord(&[Key::Control], "s");

    assert!(ui.bridge().get_review_open());
    assert_eq!(rows(&ui), vec![]);
}

#[test]
fn escape_cancels_the_review() {
    let site = editing_site();
    let ui = open_editor(&site);
    reviewing(&ui);

    ui.key(Key::Escape);

    assert!(!ui.bridge().get_review_open());
    assert_eq!(ui.pending(), 1);
    ui.chord(&[Key::Control], "z");
    assert_eq!(ui.pending(), 0, "the page has the focus again");
}

#[test]
fn enter_cancels_the_review() {
    let site = editing_site();
    let ui = open_editor(&site);
    reviewing(&ui);

    ui.key(Key::Return);

    assert!(!ui.bridge().get_review_open());
    assert_eq!(ui.pending(), 1);
}

#[test]
fn the_cancel_button_cancels() {
    let site = editing_site();
    let ui = open_editor(&site);
    reviewing(&ui);

    ui.element("Cancel", Button).invoke_accessible_default_action();

    assert!(!ui.bridge().get_review_open());
    assert_eq!(ui.pending(), 1);
}

#[test]
fn the_dialog_offers_cancel_and_save() {
    let site = editing_site();
    let ui = open_editor(&site);
    reviewing(&ui);

    assert_eq!(ui.element("Cancel", Button).accessible_enabled(), Some(true));
    assert_eq!(ui.element("Save", Button).accessible_enabled(), Some(true));
}

#[test]
fn the_review_is_rebuilt_on_each_open() {
    let site = editing_site();
    let ui = open_editor(&site);
    reviewing(&ui);
    ui.key(Key::Escape);

    ui.commit(ui.gil().key, "700");
    ui.chord(&[Key::Control], "s");

    assert!(ui.bridge().get_review_open());
    assert_eq!(rows(&ui)[1], row(ReviewKind::Field, "Gil", "Gil", "123456", "700"));
}

#[test]
fn a_blocked_page_ignores_ctrl_s() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.bridge().set_expert_warning_open(true);

    ui.bridge().invoke_review_requested();
    ui.chord(&[Key::Control], "s");

    assert!(!ui.bridge().get_review_open());
}

#[test]
fn a_default_level_edit_reviews_the_deterministic_rows() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.level().key, "34");
    ui.chord(&[Key::Control], "s");

    let shown = rows(&ui);
    assert_eq!(shown[0], row(ReviewKind::Heading, "Aria", "", "", ""));
    assert_eq!(shown.iter().filter(|row| row.kind == ReviewKind::Heading).count(), 1);
    for (label, before, after) in [
        ("Level", "31", "34"),
        ("Experience", "100100", "125357"),
        ("Accuracy gain", "143", "155"),
        ("Magic evasion gain", "133", "142"),
    ] {
        assert!(shown.contains(&row(ReviewKind::Field, "Aria", label, before, after)), "no row for {label}");
    }
}

#[test]
fn rows_added_and_removed_appear() {
    let site = editing_site();
    let ui = open_editor(&site);
    let data = ui.bridge_data();
    ui.bridge().invoke_item_add_requested(list(&data, "Items").key, 4);
    ui.bridge().invoke_item_remove_requested(row_of(&ui.bridge_data(), "Items", 10).key);

    ui.chord(&[Key::Control], "s");

    let shown = rows(&ui);
    assert!(shown.contains(&row(ReviewKind::Added, "X-Potion", "", "", "")));
    assert!(shown.contains(&row(ReviewKind::Removed, "Phoenix Down", "", "", "")));
}

#[test]
fn keys_behind_the_dialog_reach_nothing() {
    let site = editing_site();
    let ui = open_editor(&site);
    reviewing(&ui);

    ui.type_text("123");
    ui.chord(&[Key::Control], "z");

    assert!(ui.bridge().get_review_open());
    assert_eq!(ui.pending(), 1);
    assert_eq!(ui.gil().text, "500");
}

#[test]
fn the_review_does_not_move_the_navigation() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.navigate(EditorCommand::Party, 0);
    reviewing(&ui);
    ui.key(Key::Escape);

    assert_eq!(ui.bridge().get_command(), EditorCommand::Party);
    assert_eq!(ui.bridge().get_character(), 0);
}
