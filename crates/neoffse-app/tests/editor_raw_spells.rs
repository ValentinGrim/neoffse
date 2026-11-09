//! The Raw spells command: the expert-only spell pages leave Magic for a
//! command of their own, below Magic, with one button per character. The
//! command exists in expert mode on FF II alone, and the editor leaves it when
//! the section goes. Geometry comes from the accessibility tree of Slint's
//! testing backend; no window shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Rect, SLACK, Ui, editing_site, ff2_site, ff2_ui, first_rect, open_editor, resize, row_height};
use i_slint_backend_testing::AccessibleRole::{Button, TextInput};
use neoffse_app::{EditorCommand, refresh_editor};
use slint::Model;
use slint::platform::Key;

const NAMES: [&str; 4] = ["Ivo", "Mara", "Tane", "Élan"];
const RAW: &str = "Raw spells";

fn expert_on(ui: &Ui) {
    let bridge = ui.bridge();
    bridge.invoke_expert_toggle_requested();
    bridge.invoke_expert_warning_confirmed();
    assert!(ui.app.editor.expert());
}

fn expert_off(ui: &Ui) {
    ui.bridge().invoke_expert_toggle_requested();
    assert!(!ui.app.editor.expert());
}

/// An FF II editor in expert mode at 1280 by 800.
fn expert_ui(site: &common::Site) -> Ui {
    let ui = ff2_ui(site);
    resize(&ui, 1280.0, 800.0);
    expert_on(&ui);
    ui
}

fn button(ui: &Ui, label: &str) -> Rect {
    Rect::of(&ui.element(label, Button))
}

#[test]
fn default_mode_has_no_raw_spells_row() {
    let site = ff2_site();
    let ui = ff2_ui(&site);
    assert!(!ui.has_element(RAW, Button));
    assert!(ui.has_element("Magic", Button));
    assert!(!ui.bridge().get_data().raw_magic.present);
}

#[test]
fn ff1_expert_mode_adds_no_row() {
    let site = editing_site();
    let ui = open_editor(&site);
    expert_on(&ui);
    assert!(!ui.has_element(RAW, Button));
    assert!(ui.has_element("Magic", Button));
    assert!(!ui.bridge().get_data().raw_magic.present);
}

#[test]
fn expert_mode_adds_the_row_below_magic() {
    let site = ff2_site();
    let ui = expert_ui(&site);
    let (magic, raw) = (button(&ui, "Magic"), button(&ui, RAW));
    assert!(raw.y >= magic.bottom() - SLACK, "Raw spells ({raw:?}) is not below Magic ({magic:?})");
    assert!((raw.h - row_height(&ui)).abs() <= SLACK, "the row is {} px high", raw.h);
    assert!((raw.x - magic.x).abs() <= SLACK && (raw.w - magic.w).abs() <= SLACK);
}

#[test]
fn the_row_opens_the_raw_page() {
    let site = ff2_site();
    let ui = expert_ui(&site);
    ui.click_element(&ui.element(RAW, Button));
    assert_eq!(ui.bridge().get_command(), EditorCommand::RawMagic);
    assert_eq!(ui.bridge().get_character(), -1);
    for name in NAMES {
        assert!(ui.has_element(name, Button), "no button {name}");
    }
    assert!(ui.has_element("List 2, Fire, value", TextInput));
    assert!(!ui.has_element("Fire", TextInput), "the Level box shows on the raw page");

    ui.key(Key::Escape);
    assert!(ui.on_cards());
}

#[test]
fn the_magic_page_keeps_four_named_buttons_in_expert_mode() {
    let site = ff2_site();
    let ui = expert_ui(&site);
    ui.navigate(EditorCommand::Magic, -1);
    for name in NAMES {
        assert!(ui.has_element(name, Button), "no button {name}");
        assert!(!ui.has_element(&format!("{name}, raw spells"), Button), "{name} has a raw button on Magic");
    }
    assert!(ui.has_element("Fire", TextInput));
}

/// The widths of the character buttons.
fn widths(ui: &Ui) -> Vec<f32> {
    NAMES.iter().map(|name| button(ui, name).w).collect()
}

/// Both pages keep their buttons as wide at 800 by 600 as at 1280 by 800, and
/// inside the page.
fn assert_buttons_keep_their_width(ui: &Ui, command: EditorCommand) {
    ui.navigate(command, -1);
    resize(ui, 1280.0, 800.0);
    let wide = widths(ui);
    resize(ui, 800.0, 600.0);
    let narrow = widths(ui);
    let page = first_rect(ui, "CharacterPage");
    for ((name, wide), narrow) in NAMES.iter().zip(wide).zip(narrow) {
        assert!(
            (wide - narrow).abs() <= SLACK,
            "{command:?}: the {name} button is {narrow} px wide at 800 and {wide} at 1280"
        );
        let rect = button(ui, name);
        assert!(page.holds(&rect), "{command:?}: the {name} button ({rect:?}) leaves the page ({page:?})");
    }
}

#[test]
fn the_character_buttons_keep_their_width_at_800() {
    let site = ff2_site();
    let ui = expert_ui(&site);
    assert_buttons_keep_their_width(&ui, EditorCommand::Magic);
    assert_buttons_keep_their_width(&ui, EditorCommand::RawMagic);
}

#[test]
fn the_raw_row_fits_the_command_column_at_800() {
    let site = ff2_site();
    let ui = expert_ui(&site);
    ui.navigate(EditorCommand::RawMagic, -1);
    resize(&ui, 800.0, 600.0);
    let (list, status) = (first_rect(&ui, "CommandList"), first_rect(&ui, "StatusPanel"));
    let raw = button(&ui, RAW);
    assert!(list.holds(&raw), "the row ({raw:?}) leaves the list ({list:?})");
    assert!(list.bottom() <= status.y + SLACK, "the list ({list:?}) overlaps the status panel ({status:?})");
    let screen = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    let page = first_rect(&ui, "CharacterPage");
    assert!(screen.holds(&page), "the Raw spells page ({page:?}) leaves the window");
}

#[test]
fn expert_off_with_the_raw_section_open_returns_to_magic() {
    let site = ff2_site();
    let ui = expert_ui(&site);
    ui.click_element(&ui.element(RAW, Button));
    ui.click_element(&ui.element("Élan", Button));
    assert!(ui.has_element("List 2, Fire, value", TextInput));

    expert_off(&ui);
    let bridge = ui.bridge();
    assert_eq!(bridge.get_command(), EditorCommand::Magic);
    assert_eq!(bridge.get_character(), -1);
    assert!(!bridge.get_data().raw_magic.present);
    assert_eq!(bridge.get_data().magic.pages.row_count(), 4);
    assert!(ui.has_element("Fire", TextInput), "the Level page of the first character shows");
    assert!(!ui.has_element("List 2, Fire, value", TextInput));
    assert!(!ui.has_element(RAW, Button));
}

/// With no raw section, a refresh moves `RawMagic` to Magic and leaves any
/// other command alone.
fn assert_refresh_settles(ui: &Ui) {
    ui.bridge().set_command(EditorCommand::RawMagic);
    refresh_editor(&ui.window, &ui.app);
    assert_eq!(ui.bridge().get_command(), EditorCommand::Magic);

    ui.navigate(EditorCommand::Party, 2);
    refresh_editor(&ui.window, &ui.app);
    assert_eq!(ui.bridge().get_command(), EditorCommand::Party);
    assert_eq!(ui.bridge().get_character(), 2);
}

#[test]
fn a_refresh_moves_a_command_without_a_section_to_magic() {
    let site = ff2_site();
    assert_refresh_settles(&ff2_ui(&site));
}

#[test]
fn a_refresh_moves_a_command_without_a_section_to_magic_on_ff1() {
    let site = editing_site();
    assert_refresh_settles(&open_editor(&site));
}

#[test]
fn the_row_leaving_returns_the_focus() {
    let site = ff2_site();
    let ui = expert_ui(&site);
    // Home, Back, Party, Items, Magic, Raw spells.
    ui.navigate(EditorCommand::Magic, -1);
    for _ in 0..6 {
        ui.key(Key::Tab);
    }
    assert!(ui.has_element(RAW, Button));

    expert_off(&ui);
    assert!(!ui.has_element(RAW, Button));
    ui.key(Key::Escape);
    assert!(ui.on_cards(), "the focus left with the row and Escape reached nothing");
}
