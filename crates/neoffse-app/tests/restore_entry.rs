//! The ways into the Restore page and back out: the Backups button of a Home
//! game group, the File menu item, Back. The testing backend draws no menu,
//! so the menu tests call the window callback the item calls (a known gap).
//! Everything runs on temporary folders.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::Arc;

use common::{Paged, T0, editing_site, install, make_backup};
use i_slint_backend_testing::AccessibleRole::Button;
use neoffse_app::{EditorCommand, SlotKind};

#[test]
fn home_shows_a_backups_button_per_game_group() {
    let site = editing_site();
    install(&site, 1);
    let p = Paged::home(&site);
    assert_eq!(p.ui.rows().len(), 2);
    assert!(p.ui.has_element("Backups of Final Fantasy I", Button));
    // The unsupported group has its button too.
    assert!(p.ui.has_element("Backups of Final Fantasy II", Button));
}

#[test]
fn the_home_button_opens_the_page_for_its_game() {
    let site = editing_site();
    let p = Paged::home(&site);
    p.backup("before-save", 0);
    p.backup("before-save", 1);

    p.ui.element("Backups of Final Fantasy I", Button).invoke_accessible_default_action();

    assert!(p.ui.window.get_restore_open());
    assert!(!p.ui.window.get_editor_open());
    assert_eq!(p.bridge().get_game_title().as_str(), "Final Fantasy I");
    assert_eq!(p.stamps(), ["2020-09-13 12:27 UTC", "2020-09-13 12:26 UTC"]);
    assert!(!p.bridge().get_has_notice());
}

#[test]
fn an_unsupported_game_lists_its_backups() {
    let site = editing_site();
    install(&site, 1);
    let p = Paged::home(&site);
    let source = tempfile::tempdir().unwrap();
    fs::write(source.path().join("notes.txt"), [7_u8; 16]).unwrap();
    make_backup(&p.data_dir(), "ff2-pr", source.path(), &["notes.txt"], "before-save", T0, 0);

    p.ui.element("Backups of Final Fantasy II", Button).invoke_accessible_default_action();

    assert!(p.ui.window.get_restore_open());
    assert_eq!(p.bridge().get_game_title().as_str(), "Final Fantasy II");
    let rows = p.rows();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].slot_kind, SlotKind::Other);
    assert_eq!(rows[0].file.as_str(), "notes.txt");
}

#[test]
fn an_unknown_game_id_opens_nothing() {
    let site = editing_site();
    let p = Paged::home(&site);
    p.ui.window.invoke_backups_requested("ff99-pr".into());
    p.ui.window.invoke_backups_requested("".into());
    assert!(!p.ui.window.get_restore_open());
}

#[test]
fn the_button_is_disabled_while_an_open_runs() {
    let site = editing_site();
    let p = Paged::home(&site);
    let key = p.ui.first_slots()[0].key;
    p.ui.window.invoke_slot_clicked(key);
    assert!(p.ui.window.get_busy());

    assert_eq!(p.ui.element("Backups of Final Fantasy I", Button).accessible_enabled(), Some(false));
    p.ui.window.invoke_backups_requested("ff1-pr".into());
    assert!(!p.ui.window.get_restore_open(), "the callback guards too");

    p.ui.manual.run_all();
    assert!(p.ui.window.get_editor_open());
}

#[test]
fn the_file_menu_item_opens_the_open_games_backups() {
    let site = editing_site();
    let p = Paged::editing(&site);
    p.backup("before-save", 0);

    p.ui.window.invoke_restore_menu_requested();

    assert!(p.ui.window.get_restore_open());
    assert!(p.ui.window.get_editor_open(), "the page opens over the editor");
    assert!(p.ui.session_open());
    assert_eq!(p.bridge().get_game_title().as_str(), "Final Fantasy I");
    assert_eq!(p.rows().len(), 1);
}

#[test]
fn the_file_menu_does_nothing_on_home() {
    let site = editing_site();
    let p = Paged::home(&site);
    p.ui.window.invoke_restore_menu_requested();
    assert!(!p.ui.window.get_restore_open());
}

#[test]
fn the_file_menu_does_nothing_while_a_dialog_is_open() {
    let site = editing_site();
    let p = Paged::editing(&site);
    p.ui.make_dirty();
    p.ui.window.invoke_back_requested();
    assert!(p.ui.bridge().get_discard_dialog_open());

    p.ui.window.invoke_restore_menu_requested();

    assert!(!p.ui.window.get_restore_open());
}

#[test]
fn the_file_menu_does_nothing_on_the_page_itself() {
    let site = editing_site();
    let p = Paged::editing(&site);
    p.open_from_menu();
    p.bridge().set_notice(neoffse_app::RestoreNotice {
        problem: neoffse_app::RestoreProblem::Failed,
        value: "kept".into(),
    });
    p.ui.window.invoke_restore_menu_requested();
    assert_eq!(p.bridge().get_notice().value.as_str(), "kept", "the page was not rebuilt");
}

#[test]
fn the_editor_keeps_its_place_and_its_edits() {
    let site = editing_site();
    let p = Paged::editing(&site);
    p.ui.make_dirty();
    p.ui.navigate(EditorCommand::Items, 1);
    let session = p.ui.app.state.session().unwrap();

    p.open_from_menu();
    p.bridge().invoke_back_requested();

    assert!(!p.ui.window.get_restore_open());
    assert!(p.ui.window.get_editor_open());
    assert_eq!(p.ui.bridge().get_command(), EditorCommand::Items);
    assert_eq!(p.ui.bridge().get_character(), 1);
    assert_eq!(p.ui.pending(), 1);
    assert!(Arc::ptr_eq(&session, &p.ui.app.state.session().unwrap()));
    assert!(!p.ui.app.editor.leave_pending());
}

#[test]
fn back_from_home_returns_to_home() {
    let site = editing_site();
    let p = Paged::home(&site);
    p.backup("before-save", 0);
    p.open_from_home();

    p.bridge().invoke_back_requested();

    assert!(!p.ui.window.get_restore_open());
    assert!(!p.ui.window.get_editor_open());
    assert_eq!(p.ui.rows().len(), 1, "Home still lists its games");
    assert_eq!(p.rows().len(), 0, "the page forgot its rows");
    assert!(p.ui.has_element("Backups of Final Fantasy I", Button));
}

#[test]
fn back_from_the_editor_returns_to_the_editor() {
    let site = editing_site();
    let p = Paged::editing(&site);
    p.open_from_menu();
    p.bridge().invoke_back_requested();
    assert!(!p.ui.window.get_restore_open());
    assert!(p.ui.window.get_editor_open());
    assert!(p.ui.session_open());
}
