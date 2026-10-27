//! The close veto. These tests call `show()` on Slint's testing backend,
//! installed by `init_no_event_loop()` before any window exists: it has no
//! screen, so nothing opens on the desktop.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Ui, editing_site};
use neoffse_app::AboutDialog;
use slint::ComponentHandle;

/// A window with About connected, the first slot open, both windows shown.
fn shown(site: &common::Site) -> (Ui, AboutDialog) {
    let (ui, about) = Ui::with_about(site);
    ui.open_first_slot();
    ui.window.show().unwrap();
    ui.window.invoke_about_requested();
    assert!(ui.window.window().is_visible());
    assert!(about.window().is_visible());
    (ui, about)
}

fn request_quit(ui: &Ui) {
    ui.window.invoke_quit_requested();
}

#[test]
fn a_clean_close_hides_the_window_and_the_about_dialog() {
    let site = editing_site();
    let (ui, about) = shown(&site);
    ui.request_close();
    assert!(!ui.window.window().is_visible());
    assert!(!about.window().is_visible());
}

#[test]
fn a_dirty_close_is_vetoed() {
    let site = editing_site();
    let (ui, about) = shown(&site);
    ui.commit(ui.gil().key, "500");
    ui.request_close();

    assert!(ui.window.window().is_visible());
    assert!(about.window().is_visible(), "About stays while the window stays");
    assert!(ui.bridge().get_discard_dialog_open());
    assert!(ui.session_open());
}

#[test]
fn cancel_keeps_the_window() {
    let site = editing_site();
    let (ui, _about) = shown(&site);
    ui.commit(ui.gil().key, "500");
    ui.request_close();
    ui.bridge().invoke_discard_cancelled();

    assert!(ui.window.window().is_visible());
    assert!(!ui.bridge().get_discard_dialog_open());
    ui.request_close();
    assert!(ui.window.window().is_visible(), "a new close request vetoes again");
    assert!(ui.bridge().get_discard_dialog_open());
}

#[test]
fn discard_closes_the_window_and_the_session() {
    let site = editing_site();
    let (ui, about) = shown(&site);
    ui.commit(ui.gil().key, "500");
    ui.request_close();
    ui.bridge().invoke_discard_confirmed();

    assert!(!ui.window.window().is_visible());
    assert!(!about.window().is_visible());
    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(!ui.session_open());
}

#[test]
fn closing_on_home_needs_no_question() {
    let site = editing_site();
    let (ui, about) = Ui::with_about(&site);
    ui.window.show().unwrap();
    ui.window.invoke_about_requested();
    ui.request_close();

    assert!(!ui.window.window().is_visible());
    assert!(!about.window().is_visible());
}

#[test]
fn a_close_during_the_back_dialog_replaces_its_reason() {
    let site = editing_site();
    let (ui, about) = shown(&site);
    ui.commit(ui.gil().key, "500");
    ui.window.invoke_back_requested();
    ui.request_close();

    assert!(ui.window.window().is_visible());
    assert!(ui.bridge().get_discard_dialog_open());
    ui.bridge().invoke_discard_confirmed();
    assert!(!ui.window.window().is_visible(), "the reason is Close, not Back");
    assert!(!about.window().is_visible());
}

#[test]
fn quit_with_unsaved_edits_asks_first() {
    let site = editing_site();
    let (ui, about) = shown(&site);
    ui.commit(ui.gil().key, "500");
    request_quit(&ui);

    assert!(ui.window.window().is_visible());
    assert!(about.window().is_visible(), "About stays while the window stays");
    assert!(ui.bridge().get_discard_dialog_open());
    assert!(ui.session_open());

    ui.bridge().invoke_discard_confirmed();
    assert!(!ui.window.window().is_visible());
    assert!(!about.window().is_visible());
    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(!ui.session_open());
}

#[test]
fn quit_while_a_save_runs_changes_nothing() {
    let site = editing_site();
    let (ui, about) = shown(&site);
    ui.commit(ui.gil().key, "500");
    ui.bridge().set_working(true);
    request_quit(&ui);

    assert!(ui.window.window().is_visible());
    assert!(about.window().is_visible());
    assert!(!ui.bridge().get_discard_dialog_open(), "the veto is silent");
    assert!(ui.session_open());
    assert_eq!(ui.pending(), 1);

    ui.bridge().set_working(false);
    request_quit(&ui);
    assert!(ui.bridge().get_discard_dialog_open(), "the dirty session asks once the worker ends");
}

#[test]
fn a_clean_quit_hides_the_window_and_the_about_dialog() {
    let site = editing_site();
    let (ui, about) = shown(&site);
    request_quit(&ui);

    assert!(!ui.window.window().is_visible());
    assert!(!about.window().is_visible());
}

#[test]
fn quit_on_home_needs_no_question() {
    let site = editing_site();
    let (ui, about) = Ui::with_about(&site);
    ui.window.show().unwrap();
    ui.window.invoke_about_requested();
    request_quit(&ui);

    assert!(!ui.window.window().is_visible());
    assert!(!about.window().is_visible());
}
