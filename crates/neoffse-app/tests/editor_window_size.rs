//! The window returns to the size Home had when the editor closes by Back or
//! by the Home button. Sizes are set through `set_size` and read back in
//! logical pixels on Slint's testing backend; no window shows.
//!
//! The backend stores a size as given and answers at once. A real window
//! manager may refuse, clamp or delay it, so these tests prove the app asks
//! for the right size and nothing about the answer.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{ID_1, Site, Ui, editing_site, resize, slot_file, steam_slot};
use neoffse::core::Game;
use slint::ComponentHandle;

/// The window size in logical pixels.
fn logical(ui: &Ui) -> (f32, f32) {
    let size = ui.window.window().size().to_logical(ui.window.window().scale_factor());
    (size.width, size.height)
}

fn assert_size(ui: &Ui, expected: (f32, f32), what: &str) {
    let (width, height) = logical(ui);
    assert!(
        (width - expected.0).abs() < 0.5 && (height - expected.1).abs() < 0.5,
        "{what}: {width}x{height}, expected {expected:?}"
    );
}

/// Home at `home`, then the first slot open on the editor.
fn open_at(site: &Site, home: (f32, f32)) -> Ui {
    let ui = Ui::scanned(site, None);
    resize(&ui, home.0, home.1);
    ui.open_first_slot();
    ui
}

#[test]
fn back_restores_the_home_size() {
    let site = editing_site();
    let ui = open_at(&site, (900.0, 650.0));
    resize(&ui, 1100.0, 700.0);
    ui.window.invoke_back_requested();
    assert!(!ui.window.get_editor_open());
    assert_size(&ui, (900.0, 650.0), "Back restores Home's size");
}

#[test]
fn the_editor_size_is_not_kept_without_the_rule() {
    let site = editing_site();
    let ui = open_at(&site, (900.0, 650.0));
    resize(&ui, 1100.0, 700.0);
    assert_size(&ui, (1100.0, 700.0), "the resize holds while the editor is open");
    assert!(ui.window.get_editor_open());
    ui.window.invoke_back_requested();
    assert_ne!(logical(&ui), (1100.0, 700.0), "the restore changes the size, no missing resize");
}

#[test]
fn the_home_button_restores_it() {
    let site = editing_site();
    let ui = open_at(&site, (900.0, 650.0));
    resize(&ui, 1100.0, 700.0);
    ui.window.invoke_home_requested();
    assert!(!ui.window.get_editor_open());
    assert_size(&ui, (900.0, 650.0), "the Home button restores Home's size");
}

#[test]
fn an_unchanged_size_stays() {
    let site = editing_site();
    let ui = open_at(&site, (900.0, 650.0));
    ui.window.invoke_back_requested();
    assert!(!ui.window.get_editor_open());
    assert_size(&ui, (900.0, 650.0), "no resize, no change");
}

#[test]
fn a_size_set_on_home_after_back_is_the_next_record() {
    let site = editing_site();
    let ui = open_at(&site, (900.0, 650.0));
    resize(&ui, 1100.0, 700.0);
    ui.window.invoke_back_requested();

    resize(&ui, 1000.0, 640.0);
    ui.open_first_slot();
    resize(&ui, 1200.0, 760.0);
    ui.window.invoke_back_requested();
    assert_size(&ui, (1000.0, 640.0), "the record is replaced, not the first one kept");
}

#[test]
fn a_failed_open_records_nothing() {
    let site = editing_site();
    steam_slot(&site, 0, ID_1, slot_file(2), b"not a save");
    let ui = Ui::scanned(&site, None);
    let slots = ui.first_slots();
    let broken = slots.iter().find(|slot| !slot.summarized).expect("a slot that is not a save");
    let good = slots.iter().find(|slot| slot.summarized).expect("a good slot");

    resize(&ui, 900.0, 650.0);
    ui.window.invoke_slot_clicked(broken.key);
    ui.manual.run_all();
    assert!(ui.window.get_has_open_problem(), "the open fails");
    assert!(!ui.window.get_editor_open());

    resize(&ui, 1000.0, 640.0);
    ui.window.invoke_slot_clicked(good.key);
    ui.manual.run_all();
    assert!(ui.window.get_editor_open(), "the good slot opens");
    resize(&ui, 1200.0, 760.0);
    ui.window.invoke_back_requested();
    assert_size(&ui, (1000.0, 640.0), "the size at the successful opening");
}

#[test]
fn a_dirty_back_keeps_the_editor_size_until_the_discard() {
    let site = editing_site();
    let ui = open_at(&site, (900.0, 650.0));
    ui.make_dirty();
    resize(&ui, 1100.0, 700.0);
    ui.window.invoke_back_requested();
    assert!(ui.bridge().get_discard_dialog_open(), "the discard dialog asks");
    assert!(ui.window.get_editor_open());
    assert_size(&ui, (1100.0, 700.0), "the editor keeps its size while the dialog asks");
}

#[test]
fn the_restore_page_from_home_neither_records_nor_restores() {
    let site = editing_site();
    let ui = Ui::scanned(&site, None);
    resize(&ui, 900.0, 650.0);
    ui.window.invoke_backups_requested(Game::Ff1Pr.id().into());
    assert!(ui.window.get_restore_open());
    resize(&ui, 1100.0, 700.0);
    ui.restore_bridge().invoke_back_requested();
    assert!(!ui.window.get_restore_open());
    assert!(!ui.window.get_editor_open());
    assert_size(&ui, (1100.0, 700.0), "Home keeps the size set on the page");

    // The page recorded nothing: an editor opened and closed later restores its own record.
    ui.open_first_slot();
    resize(&ui, 1200.0, 760.0);
    ui.window.invoke_back_requested();
    assert_size(&ui, (1100.0, 700.0), "the record comes from the opening");
}

#[test]
fn the_restore_page_from_the_editor_keeps_the_record() {
    let site = editing_site();
    let ui = open_at(&site, (900.0, 650.0));
    resize(&ui, 1100.0, 700.0);
    ui.window.invoke_restore_menu_requested();
    assert!(ui.window.get_restore_open());
    resize(&ui, 1000.0, 680.0);
    ui.restore_bridge().invoke_back_requested();
    assert!(!ui.window.get_restore_open());
    assert!(ui.window.get_editor_open(), "the editor stays open");
    assert_size(&ui, (1000.0, 680.0), "the page's Back restores nothing");

    ui.window.invoke_back_requested();
    assert!(!ui.window.get_editor_open());
    assert_size(&ui, (900.0, 650.0), "Home's size from the opening");
}

#[test]
fn a_maximised_window_records_and_restores_nothing() {
    let site = editing_site();
    let ui = Ui::scanned(&site, None);
    resize(&ui, 900.0, 650.0);
    ui.window.window().set_maximized(true);
    assert!(ui.window.window().is_maximized(), "the testing backend keeps the maximised flag");
    ui.open_first_slot();
    ui.window.window().set_maximized(false);
    resize(&ui, 1100.0, 700.0);
    ui.window.invoke_back_requested();
    assert_size(&ui, (1100.0, 700.0), "a maximised opening records nothing");
}

#[test]
fn a_window_maximised_at_the_close_is_left_alone() {
    let site = editing_site();
    let ui = open_at(&site, (900.0, 650.0));
    resize(&ui, 1100.0, 700.0);
    ui.window.window().set_maximized(true);
    assert!(ui.window.window().is_maximized(), "the testing backend keeps the maximised flag");
    ui.window.invoke_back_requested();
    assert_size(&ui, (1100.0, 700.0), "a maximised close restores nothing");
    ui.window.window().set_maximized(false);
    resize(&ui, 1000.0, 640.0);
    ui.open_first_slot();
    resize(&ui, 1200.0, 760.0);
    ui.window.invoke_back_requested();
    assert_size(&ui, (1000.0, 640.0), "the record was forgotten at the maximised close");
}
