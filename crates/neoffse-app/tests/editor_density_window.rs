//! The editor page at the dense text size: its fields and buttons match
//! Home's height, nothing leaves the window at 800 by 600 or 1280 by 800, and
//! the left column gives the page the room it no longer needs. Geometry comes
//! from the accessibility tree of Slint's testing backend; no window shows.
//!
//! The backend enforces no window minimum, so the 800 by 600 minimum of
//! `ui/app.slint` is not observable here; a real window checks it.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Rect, SLACK, Ui, editing_site, first_rect, font_body, open_editor, resize, row_height};
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use neoffse_app::EditorCommand;

const SIZES: [(f32, f32); 2] = [(800.0, 600.0), (1280.0, 800.0)];

fn rect_labelled(ui: &Ui, label: &str) -> Rect {
    Rect::of(&ui.element(label, AccessibleRole::Button))
}

/// The box labelled `label`: its `CommitEdit`, which holds the text input.
fn box_height(ui: &Ui, label: &str) -> f32 {
    let input = Rect::of(&ui.input(label));
    ElementHandle::find_by_element_type_name(&ui.window, "CommitEdit")
        .map(|edit| Rect::of(&edit))
        .filter(|rect| rect.holds(&input))
        .map(|rect| rect.h)
        .min_by(f32::total_cmp)
        .unwrap_or_else(|| panic!("no box holds the input labelled {label}"))
}

fn assert_inside(window: (f32, f32), what: &str, rect: &Rect) {
    let screen = Rect {
        x: 0.0,
        y: 0.0,
        w: window.0,
        h: window.1,
    };
    assert!(screen.holds(rect), "{}x{}: {what} ({rect:?}) leaves the window", window.0, window.1);
}

#[test]
fn the_editor_matches_the_height_of_home() {
    let site = editing_site();
    let ui = Ui::scanned(&site, None);
    resize(&ui, 1280.0, 800.0);
    let home = rect_labelled(&ui, "Refresh").h;

    ui.window.invoke_slot_clicked(ui.first_slots()[0].key);
    ui.manual.run_all();
    assert!(ui.window.get_editor_open());
    ui.make_dirty();
    ui.navigate(EditorCommand::Party, 0);

    let level = box_height(&ui, "Level");
    let undo = rect_labelled(&ui, "Undo").h;
    assert!((level - home).abs() <= 2.0, "the Level box is {level} px high, Home's Refresh is {home}");
    assert!((undo - home).abs() <= 2.0, "Undo is {undo} px high, Home's Refresh is {home}");
}

#[test]
fn the_command_rows_are_one_row_high() {
    let site = editing_site();
    let ui = open_editor(&site);
    let reference = row_height(&ui);
    for label in ["Party", "Items", "Magic"] {
        let height = rect_labelled(&ui, label).h;
        assert!((height - reference).abs() <= SLACK, "the {label} row is {height} px high, a row is {reference}");
    }
}

#[test]
fn nothing_leaves_the_window() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_notice();
    for (command, character, page) in [
        (EditorCommand::Party, -1, "PartyCards"),
        (EditorCommand::Items, -1, "InventoryPage"),
        (EditorCommand::Magic, -1, "CharacterPage"),
    ] {
        ui.navigate(command, character);
        for size in SIZES {
            resize(&ui, size.0, size.1);
            let list = first_rect(&ui, "CommandList");
            let status = first_rect(&ui, "StatusPanel");
            for (what, rect) in [
                ("the command list", list),
                ("the status panel", status),
                (page, first_rect(&ui, page)),
                ("the pending bar", first_rect(&ui, "PendingBar")),
            ] {
                assert_inside(size, what, &rect);
            }
            assert!(
                list.bottom() <= status.y + SLACK,
                "{}x{}: the command list ({list:?}) overlaps the status panel ({status:?})",
                size.0,
                size.1
            );
        }
    }
}

#[test]
fn the_bar_buttons_fit_at_800() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.level().key, "34");
    resize(&ui, 800.0, 600.0);
    let bar = first_rect(&ui, "PendingBar");
    let buttons: Vec<Rect> = ["Undo", "Redo", "Review & save"].into_iter().map(|label| rect_labelled(&ui, label)).collect();
    for (label, button) in ["Undo", "Redo", "Review & save"].into_iter().zip(&buttons) {
        assert_inside((800.0, 600.0), label, button);
        assert!(bar.holds(button), "{label} ({button:?}) leaves the bar ({bar:?})");
    }
    for pair in buttons.windows(2) {
        assert!(pair[0].right() <= pair[1].x + SLACK, "two bar buttons overlap ({:?}, {:?})", pair[0], pair[1]);
    }
    let text = ElementHandle::find_by_element_type_name(&ui.window, "PendingBar")
        .flat_map(|bar| bar.query_descendants().match_accessible_role(AccessibleRole::Text).find_all())
        .map(|text| Rect::of(&text))
        .next()
        .expect("the bar text");
    assert!(text.x < buttons[0].x, "the bar text ({text:?}) does not start left of Undo ({:?})", buttons[0]);
    assert!(
        text.right() <= buttons[0].x + SLACK,
        "the bar text ({text:?}) runs under Undo ({:?})",
        buttons[0]
    );
}

#[test]
fn the_left_column_is_narrower_than_before() {
    let site = editing_site();
    let ui = open_editor(&site);
    let em = font_body(&ui);
    let status_width = |ui: &Ui| first_rect(ui, "StatusPanel").w;

    resize(&ui, 1280.0, 800.0);
    let wide = first_rect(&ui, "CommandList").w;
    assert!(wide <= 15.0 * em + SLACK, "the column is {wide} px at 1280, 15 em are {}", 15.0 * em);
    assert!(
        (wide - status_width(&ui)).abs() <= SLACK,
        "the command list and the status panel differ in width"
    );
    let panel = first_rect(&ui, "PartyCards");
    assert!(panel.w >= 1000.0, "the page is {} px wide at 1280", panel.w);

    // A fifth of the width, between 10 and 15 em: the preferred width decides.
    resize(&ui, 800.0, 600.0);
    let narrow = first_rect(&ui, "CommandList").w;
    assert!((narrow - 0.2 * 800.0).abs() <= SLACK, "the column is {narrow} px at 800, a fifth is 160");
    resize(&ui, 500.0, 600.0);
    let least = first_rect(&ui, "CommandList").w;
    assert!(least >= 10.0 * em - SLACK, "the column is {least} px at 500, 10 em are {}", 10.0 * em);
}
