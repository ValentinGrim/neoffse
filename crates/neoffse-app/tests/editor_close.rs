//! The close veto. These tests call `show()` on Slint's testing backend,
//! installed by `init_no_event_loop()` before any window exists: it has no
//! screen, so nothing opens on the desktop.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Ui, editing_site};
use neoffse_app::AboutDialog;
use slint::ComponentHandle;
use slint::platform::WindowEvent;

/// A window with About connected, the first slot open, both windows shown.
fn shown(site: &common::Site) -> (Ui, AboutDialog) {
    let (ui, about) = Ui::with_about(site);
    let key = ui.first_slots()[0].key;
    ui.window.invoke_slot_clicked(key);
    ui.manual.run_all();
    assert!(ui.window.get_editor_open());
    ui.window.show().unwrap();
    ui.window.invoke_about_requested();
    assert!(ui.window.window().is_visible());
    assert!(about.window().is_visible());
    (ui, about)
}

fn request_close(ui: &Ui) {
    ui.window.window().dispatch_event(WindowEvent::CloseRequested);
}

#[test]
fn a_clean_close_hides_the_window_and_the_about_dialog() {
    let site = editing_site();
    let (ui, about) = shown(&site);
    request_close(&ui);
    assert!(!ui.window.window().is_visible());
    assert!(!about.window().is_visible());
}

#[test]
fn a_dirty_close_is_vetoed() {
    let site = editing_site();
    let (ui, about) = shown(&site);
    ui.commit(ui.gil().key, "500");
    request_close(&ui);

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
    request_close(&ui);
    ui.bridge().invoke_discard_cancelled();

    assert!(ui.window.window().is_visible());
    assert!(!ui.bridge().get_discard_dialog_open());
    request_close(&ui);
    assert!(ui.window.window().is_visible(), "a new close request vetoes again");
    assert!(ui.bridge().get_discard_dialog_open());
}

#[test]
fn discard_closes_the_window_and_the_session() {
    let site = editing_site();
    let (ui, about) = shown(&site);
    ui.commit(ui.gil().key, "500");
    request_close(&ui);
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
    request_close(&ui);

    assert!(!ui.window.window().is_visible());
    assert!(!about.window().is_visible());
}

#[test]
fn a_close_during_the_back_dialog_replaces_its_reason() {
    let site = editing_site();
    let (ui, about) = shown(&site);
    ui.commit(ui.gil().key, "500");
    ui.window.invoke_back_requested();
    request_close(&ui);

    assert!(ui.window.window().is_visible());
    assert!(ui.bridge().get_discard_dialog_open());
    ui.bridge().invoke_discard_confirmed();
    assert!(!ui.window.window().is_visible(), "the reason is Close, not Back");
    assert!(!about.window().is_visible());
}
