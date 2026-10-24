//! The running-game warning of a restore. The probe is scripted; nothing
//! scans the real process list and every write goes to a temporary folder.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::atomic::Ordering;

use common::{Machine, Paged, Site, editing_site, listing, rig, slot_bytes, slot_file};
use i_slint_backend_testing::AccessibleRole::{Button, Text};
use neoffse::core::Game;
use neoffse_app::RestoreProblem;
use slint::platform::Key;

const ROW_BUTTON: &str = "Restore the backup of 2020-09-13 12:26 UTC";
const WARNING: &str = "The game is running. Restoring now may be lost or overwritten. Restore anyway?";

/// The warning open over the page: the backup holds `slot_bytes(1)`, the
/// folder holds `slot_bytes(2)`, the game runs.
fn warned() -> (Site, Paged) {
    let site = editing_site();
    let p = Paged::home(&site);
    p.backup("before-save", 0);
    fs::write(p.folder.join(slot_file(1)), slot_bytes(2)).unwrap();
    p.game_runs(true);
    p.open_from_home();
    p.ui.element(ROW_BUTTON, Button).invoke_accessible_default_action();
    p.ui.element("Restore", Button).invoke_accessible_default_action();
    p.ui.manual.run_all();
    assert!(p.bridge().get_game_running_open());
    (site, p)
}

fn file(p: &Paged) -> Vec<u8> {
    fs::read(p.folder.join(slot_file(1))).unwrap()
}

#[test]
fn a_running_game_opens_the_warning() {
    let (_site, p) = warned();
    let bridge = p.bridge();
    assert!(bridge.get_game_running_open());
    assert!(!bridge.get_working());
    assert!(bridge.get_blocked());
    assert!(!bridge.get_has_notice());
    assert!(!bridge.get_confirm_open());
    assert_eq!(file(&p), slot_bytes(2));
    assert_eq!(listing(&p.data_dir(), Game::Ff1Pr).len(), 1, "no safety backup");
    assert!(p.ui.has_element(WARNING, Text));
    assert_eq!(p.ui.element("Restore anyway", Button).accessible_enabled(), Some(true));
}

/// The warning closes and the page has the focus again: Escape goes back.
fn assert_dismissed(p: &Paged) {
    assert!(!p.bridge().get_game_running_open());
    assert!(!p.bridge().get_working());
    assert_eq!(file(p), slot_bytes(2));
    assert_eq!(listing(&p.data_dir(), Game::Ff1Pr).len(), 1);
    assert!(p.ui.window.get_restore_open());
    p.ui.key(Key::Escape);
    assert!(!p.ui.window.get_restore_open(), "the page took the focus back");
}

#[test]
fn escape_cancels_the_warning() {
    let (_site, p) = warned();
    p.ui.key(Key::Escape);
    assert_dismissed(&p);
}

#[test]
fn enter_cancels_the_warning() {
    let (_site, p) = warned();
    p.ui.key(Key::Return);
    assert_dismissed(&p);
}

#[test]
fn cancel_keeps_the_selection_and_nothing_is_written() {
    let (_site, p) = warned();
    p.ui.element("Cancel", Button).invoke_accessible_default_action();

    assert!(!p.bridge().get_game_running_open());
    assert_eq!(file(&p), slot_bytes(2));
    assert!(p.ui.app.restore.selected().is_some(), "the selection survives the warning");

    // The same selection restores once the game has closed.
    p.game_runs(false);
    p.bridge().invoke_restore_confirmed();
    p.ui.manual.run_all();
    assert_eq!(file(&p), slot_bytes(1));
    assert_eq!(p.bridge().get_notice().problem, RestoreProblem::Restored);
}

#[test]
fn restore_anyway_restores_without_asking_again() {
    let (_site, p) = warned();
    assert_eq!(p.probes.load(Ordering::SeqCst), 1);
    p.ui.element("Restore anyway", Button).invoke_accessible_default_action();
    assert!(p.bridge().get_working());
    assert!(!p.bridge().get_game_running_open());
    p.ui.manual.run_all();

    assert_eq!(file(&p), slot_bytes(1));
    assert_eq!(p.probes.load(Ordering::SeqCst), 1, "the probe ran once in total");
    assert_eq!(p.bridge().get_notice().problem, RestoreProblem::Restored);
    assert!(!p.bridge().get_game_running_open());
    assert!(!p.bridge().get_working());
}

#[test]
fn the_warning_names_no_path() {
    let (_site, p) = warned();
    let folder = p.folder.to_string_lossy().into_owned();
    assert!(!WARNING.contains(&folder));
    let bridge = p.bridge();
    assert!(!bridge.get_game_title().contains(&folder));
    assert!(!bridge.get_pending_stamp().contains(&folder));
    assert!(!bridge.get_notice().value.contains(&folder));
}

#[test]
fn the_menu_path_shares_the_probe_counter() {
    let site = editing_site();
    let rig = rig(&site, Machine::GameRuns);
    rig.backup_now("before-save");
    rig.open_page();
    let stamp = "2020-09-13 12:26 UTC";
    rig.ui
        .element(&format!("Restore the backup of {stamp}"), Button)
        .invoke_accessible_default_action();
    rig.ui.element("Restore", Button).invoke_accessible_default_action();
    rig.ui.manual.run_all();

    assert!(rig.ui.restore_bridge().get_game_running_open());
    assert_eq!(rig.probes.load(Ordering::SeqCst), 1);
    assert_eq!(rig.backups(), 1);
}
