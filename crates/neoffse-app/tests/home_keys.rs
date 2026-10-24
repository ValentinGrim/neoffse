//! The Home screen from the keyboard: Tab reaches the toolbar, the Backups
//! button of the game group and then the slot rows, and Enter or Space opens
//! the focused slot. Key events go through `Window::dispatch_event` on Slint's
//! testing backend. No window shows and no event loop runs.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{ID_1, Site, Ui, slot_bytes, slot_file, steam_slot};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, SharedString};

fn press(ui: &Ui, text: impl Into<SharedString>) {
    let text = text.into();
    ui.window.window().dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    ui.window.window().dispatch_event(WindowEvent::KeyReleased { text });
}

/// Home with one FF I slot, scanned, and Tab pressed `tabs` times.
fn tabbed(tabs: usize) -> (Ui, Site) {
    let site = Site::new();
    steam_slot(&site, 0, ID_1, slot_file(1), &slot_bytes(1));
    let ui = Ui::scanned(&site, None);
    for _ in 0..tabs {
        press(&ui, SharedString::from(Key::Tab));
    }
    (ui, site)
}

/// Tab stops before the first slot: the owned-only check, Refresh, Open a file
/// and the Backups button of the first game group.
const TOOLBAR_STOPS: usize = 4;

#[test]
fn enter_opens_the_first_slot_after_the_toolbar() {
    let (ui, _site) = tabbed(TOOLBAR_STOPS + 1);
    press(&ui, SharedString::from(Key::Return));
    ui.manual.run_all();
    assert!(ui.window.get_editor_open());
}

#[test]
fn space_opens_the_focused_slot() {
    let (ui, _site) = tabbed(TOOLBAR_STOPS + 1);
    press(&ui, " ");
    ui.manual.run_all();
    assert!(ui.window.get_editor_open());
}

#[test]
fn the_toolbar_comes_before_the_slots() {
    let (ui, _site) = tabbed(TOOLBAR_STOPS);
    press(&ui, SharedString::from(Key::Return));
    ui.manual.run_all();
    assert!(!ui.window.get_editor_open(), "the last stop before the slots is a button, not a slot");
}

#[test]
fn the_filter_check_toggles_on_space() {
    let (ui, _site) = tabbed(1);
    assert!(ui.window.get_owned_only());
    press(&ui, " ");
    assert!(!ui.window.get_owned_only());
}

/// Home with FF I slots 1 to 3 in one folder, scanned, and Tab pressed `tabs` times.
fn tabbed_three(tabs: usize) -> (Ui, Site) {
    let site = Site::new();
    for id in 1..=3 {
        steam_slot(&site, 0, ID_1, slot_file(id), &slot_bytes(i64::from(id)));
    }
    let ui = Ui::scanned(&site, None);
    for _ in 0..tabs {
        press(&ui, SharedString::from(Key::Tab));
    }
    (ui, site)
}

/// Opens the card at Tab stop `card` after the toolbar and checks its slot number.
fn opens_slot_at_card(card: usize) {
    let (ui, _site) = tabbed_three(TOOLBAR_STOPS + card);
    press(&ui, SharedString::from(Key::Return));
    ui.manual.run_all();
    assert!(ui.window.get_editor_open(), "card {card} opens");
    assert_eq!(
        ui.window.get_editor_summary().number,
        i32::try_from(card).unwrap(),
        "Tab stop {card} is slot {card}"
    );
}

#[test]
fn the_first_card_is_slot_one() {
    opens_slot_at_card(1);
}

#[test]
fn the_second_card_is_slot_two() {
    opens_slot_at_card(2);
}

#[test]
fn the_third_card_is_slot_three() {
    opens_slot_at_card(3);
}
