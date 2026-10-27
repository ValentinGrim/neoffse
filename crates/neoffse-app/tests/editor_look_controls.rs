//! The controls of the Pixel Remaster editor: three bands on a button and a
//! text box, one height for both, the frame kept when a control is disabled.
//! Home keeps the flat frame. Geometry comes from the accessibility tree of
//! Slint's testing backend; no window shows. The backend does not expose
//! `border-radius`, so the offscreen renders show the rounding.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Rect, SLACK, Ui, editing_site, open_editor, resize};
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use neoffse_app::{EditorCommand, Theme};
use slint::ComponentHandle;

const BAND: &str = "ControlFrame::band";

/// The bands under `element`, widest first.
fn bands_of(element: &ElementHandle) -> Vec<Rect> {
    let mut bands: Vec<Rect> = element.query_descendants().match_id(BAND).find_all().iter().map(Rect::of).collect();
    bands.sort_by(|a, b| b.w.total_cmp(&a.w));
    bands
}

/// The smallest element of `type_name` that holds `inside`.
fn holder(ui: &Ui, type_name: &str, inside: &Rect) -> ElementHandle {
    ElementHandle::find_by_element_type_name(&ui.window, type_name)
        .filter(|candidate| Rect::of(candidate).holds(inside))
        .min_by(|a, b| Rect::of(a).w.total_cmp(&Rect::of(b).w))
        .unwrap_or_else(|| panic!("no {type_name} holds {inside:?}"))
}

fn button_named(ui: &Ui, label: &str) -> ElementHandle {
    holder(ui, "ThemedButton", &Rect::of(&ui.element(label, AccessibleRole::Button)))
}

fn level_box(ui: &Ui) -> ElementHandle {
    holder(ui, "ThemedEdit", &Rect::of(&ui.input("Level")))
}

fn assert_three_apart(what: &str, frame: &Rect, bands: &[Rect]) {
    assert_eq!(bands.len(), 3, "{what} draws {} bands", bands.len());
    for (k, band) in (0_u8..).map(f32::from).zip(bands) {
        assert!(
            (band.x - (frame.x + k)).abs() <= SLACK
                && (band.y - (frame.y + k)).abs() <= SLACK
                && (band.w - (frame.w - 2.0 * k)).abs() <= SLACK
                && (band.h - (frame.h - 2.0 * k)).abs() <= SLACK,
            "{what}: band {k} is {band:?} in {frame:?}"
        );
    }
}

fn on_character() -> Ui {
    let site = editing_site();
    let ui = open_editor(&site);
    resize(&ui, 1280.0, 800.0);
    ui.navigate(EditorCommand::Party, 0);
    ui.make_dirty();
    ui
}

#[test]
fn a_button_draws_three_bands() {
    let ui = on_character();
    let undo = button_named(&ui, "Undo");
    assert_three_apart("the Undo button", &Rect::of(&undo), &bands_of(&undo));
}

#[test]
fn a_disabled_button_keeps_its_frame() {
    let ui = on_character();
    // Back on the cards asks to discard: the dialog greys the bar.
    ui.navigate(EditorCommand::Party, -1);
    ui.window.invoke_back_requested();
    assert!(ui.bridge().get_discard_dialog_open());
    let redo = ui.element("Redo", AccessibleRole::Button);
    assert_eq!(redo.accessible_enabled(), Some(false));
    let redo = button_named(&ui, "Redo");
    assert_three_apart("the greyed Redo button", &Rect::of(&redo), &bands_of(&redo));
}

#[test]
fn a_text_box_draws_three_bands() {
    let ui = on_character();
    let edit = level_box(&ui);
    let frame = Rect::of(&edit);
    assert_three_apart("the Level box", &frame, &bands_of(&edit));
    let input = Rect::of(&ui.input("Level"));
    assert!(input.x >= frame.x + 3.0 - SLACK, "the text starts {} px in", input.x - frame.x);
}

#[test]
fn a_box_and_a_button_stay_one_height() {
    let ui = on_character();
    let (edit, button) = (Rect::of(&level_box(&ui)), Rect::of(&button_named(&ui, "Undo")));
    assert!((edit.h - button.h).abs() <= SLACK, "a box is {} px high, a button {}", edit.h, button.h);
    let reference = ui.window.global::<Theme>().get_look().row_height;
    assert!(
        (edit.h - reference).abs() <= 2.0,
        "the box is {} px high against the {reference} px row height",
        edit.h
    );
}

#[test]
fn the_shell_buttons_stay_flat() {
    let site = editing_site();
    let ui = Ui::scanned(&site, None);
    resize(&ui, 800.0, 600.0);
    let refresh = button_named(&ui, "Refresh");
    assert!(
        refresh.query_descendants().match_type_name("ControlFrame").find_first().is_some(),
        "the button draws a frame"
    );
    assert!(bands_of(&refresh).is_empty(), "the Home button draws a band");
}
