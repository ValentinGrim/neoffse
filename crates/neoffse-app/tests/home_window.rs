//! The `MainWindow` wired to the Home view models, on Slint's testing
//! backend. No window shows and no event loop runs: the manual dispatcher
//! drives every result.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::sync::Arc;

use common::{ID_1, ID_2, Site, Ui, install, items, slot_bytes, slot_file, steam_slot};
use neoffse_app::OpenProblem;
use slint::{ComponentHandle, Model};

const FF1: usize = 0;
const FF2: usize = 1;

fn one_slot() -> Site {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    site
}

#[test]
fn the_window_starts_empty_and_scanning_starts_on_connect() {
    let site = one_slot();
    let ui = Ui::new(&site, None);
    let window = &ui.window;
    assert_eq!(window.get_games().row_count(), 0);
    assert!(window.get_scanning());
    assert!(window.get_owned_only());
    assert!(!window.get_editor_open());
    assert!(!window.get_has_open_problem());
    assert!(!window.get_scan_failed());
    assert_eq!(ui.manual.queued_jobs(), 1);
}

#[test]
fn a_finished_scan_fills_the_games() {
    let site = one_slot();
    let ui = Ui::scanned(&site, None);
    assert!(!ui.window.get_scanning());
    assert_eq!(ui.window.get_skipped_count(), 0);
    let rows = ui.rows();
    assert_eq!(rows.len(), 1);
    let slots = ui.first_slots();
    assert_eq!(slots[0].summary.party.as_str(), "Aria, Bram, Cleo, Zoé");
}

#[test]
fn toggling_the_filter_shows_all_six() {
    let site = one_slot();
    let ui = Ui::scanned(&site, None);
    ui.window.invoke_owned_only_toggled(false);
    assert_eq!(ui.rows().len(), 6);
    assert!(!ui.window.get_owned_only());
    ui.window.invoke_owned_only_toggled(true);
    assert_eq!(ui.rows().len(), 1);
    assert!(ui.window.get_owned_only());
}

#[test]
fn a_slot_click_opens_the_editor_page() {
    let site = one_slot();
    let ui = Ui::scanned(&site, None);
    let slot = ui.first_slots().remove(0);
    ui.window.invoke_slot_clicked(slot.key);
    assert!(ui.window.get_busy());
    assert!(!ui.window.get_editor_open());

    ui.manual.run_all();
    assert!(ui.window.get_editor_open());
    assert_eq!(ui.window.get_editor_summary(), slot.summary);
    assert!(!ui.window.get_busy());
    assert!(!ui.window.get_has_open_problem());
    assert!(ui.app.editor.summary().is_some());
    assert!(!ui.window.window().is_visible());
}

#[test]
fn back_returns_to_home_and_drops_the_session() {
    let site = one_slot();
    let ui = Ui::scanned(&site, None);
    let key = ui.first_slots()[0].key;
    ui.window.invoke_slot_clicked(key);
    ui.manual.run_all();
    assert!(ui.window.get_editor_open());
    let weak = Arc::downgrade(&ui.app.state.session().expect("a session is open"));

    ui.window.invoke_back_requested();
    assert!(!ui.window.get_editor_open());
    assert!(ui.app.editor.summary().is_none());
    assert!(!ui.app.state.is_open());
    assert!(weak.upgrade().is_none(), "the session outlived Back");
    assert!(!ui.window.window().is_visible());
}

#[test]
fn a_failed_open_shows_a_problem_and_stays_home() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), b"not a save");
    let ui = Ui::scanned(&site, None);
    let key = ui.first_slots()[0].key;
    ui.window.invoke_slot_clicked(key);
    ui.manual.run_all();
    assert!(ui.window.get_has_open_problem());
    assert_eq!(ui.window.get_open_problem(), OpenProblem::NotASave);
    assert!(!ui.window.get_editor_open());
    assert!(!ui.window.get_busy());

    ui.window.invoke_problem_dismissed();
    assert!(!ui.window.get_has_open_problem());

    ui.window.invoke_slot_clicked(9999);
    assert_eq!(ui.manual.queued_jobs(), 0);
    assert!(ui.window.get_has_open_problem());
    assert_eq!(ui.window.get_open_problem(), OpenProblem::UnknownSlot);
}

#[test]
fn an_unsupported_game_cannot_open() {
    let site = Site::new();
    steam_slot(&site, FF2, ID_1, slot_file(1), &slot_bytes(1));
    let ui = Ui::scanned(&site, None);
    let rows = ui.rows();
    let row = rows.iter().find(|row| row.title.as_str() == "Final Fantasy II").expect("FF II is owned");
    let slot = items(&row.slots).remove(0);
    assert!(!slot.openable);

    ui.window.invoke_slot_clicked(slot.key);
    assert_eq!(ui.manual.queued_jobs(), 0);
    assert!(ui.window.get_has_open_problem());
    assert_eq!(ui.window.get_open_problem(), OpenProblem::Unsupported);
    assert!(!ui.window.get_editor_open());
}

#[test]
fn open_file_opens_the_picked_save() {
    let site = Site::new();
    let dir = steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let ui = Ui::scanned(&site, Some(dir.join(slot_file(1))));
    ui.window.invoke_open_file_requested();
    assert!(ui.window.get_busy());
    ui.manual.run_all();
    assert!(ui.window.get_editor_open());
    assert_eq!(ui.picker.calls().len(), 1);
    assert!(!ui.window.get_busy());
}

#[test]
fn a_cancelled_dialog_changes_nothing() {
    let site = one_slot();
    let ui = Ui::scanned(&site, None);
    ui.window.invoke_open_file_requested();
    ui.manual.run_all();
    assert_eq!(ui.picker.calls().len(), 1);
    assert!(!ui.window.get_editor_open());
    assert!(!ui.window.get_has_open_problem());
    assert!(!ui.window.get_busy());
}

#[test]
fn a_rescan_shows_new_saves() {
    let site = one_slot();
    let ui = Ui::scanned(&site, None);
    steam_slot(&site, FF1, ID_2, slot_file(2), &slot_bytes(2));
    ui.window.invoke_rescan_requested();
    assert!(ui.window.get_scanning());
    ui.manual.run_all();
    assert!(!ui.window.get_scanning());
    let folders = items(&ui.rows()[0].folders);
    assert_eq!(folders.len(), 2);
}

#[test]
fn an_install_alone_lists_the_game() {
    let site = Site::new();
    install(&site, FF1);
    let ui = Ui::scanned(&site, None);
    let rows = ui.rows();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].no_save);
}

#[test]
fn a_result_after_the_window_is_gone_is_harmless() {
    let site = one_slot();
    let Ui { window, manual, .. } = Ui::new(&site, None);
    drop(window);
    manual.run_all();
    assert_eq!(manual.queued_jobs() + manual.queued_posts(), 0);
}

#[test]
fn no_window_is_ever_shown() {
    let site = one_slot();
    let ui = Ui::scanned(&site, None);
    ui.window.invoke_owned_only_toggled(false);
    ui.window.invoke_rescan_requested();
    ui.manual.run_all();
    assert!(!ui.window.window().is_visible());
}
