//! The fields, the pages and the picker at the dense text size: one height
//! for every box, a number box that keeps its width, rows that overlap
//! nothing and Items rows that fit 800 px. Geometry comes from the
//! accessibility tree of Slint's testing backend; no window shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Part, Rect, SLACK, Ui, editing_site, lists, open_editor, resize, rows, section};
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use neoffse_app::{EditorCommand, Theme};
use slint::ComponentHandle;

const SMALL: (f32, f32) = (800.0, 600.0);
const LARGE: (f32, f32) = (1280.0, 800.0);

/// The editor on `command` and `character` at the `window` size.
fn page_ui(command: EditorCommand, character: i32, window: (f32, f32)) -> Ui {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.navigate(command, character);
    resize(&ui, window.0, window.1);
    ui
}

fn rects(ui: &Ui, type_name: &str) -> Vec<Rect> {
    ElementHandle::find_by_element_type_name(&ui.window, type_name)
        .map(|element| Rect::of(&element))
        .collect()
}

fn the_rect(ui: &Ui, type_name: &str) -> Rect {
    *rects(ui, type_name).first().unwrap_or_else(|| panic!("no element of type {type_name}"))
}

fn row_height(ui: &Ui) -> f32 {
    ui.window.global::<Theme>().get_look().row_height
}

fn font_body(ui: &Ui) -> f32 {
    ui.window.global::<Theme>().get_look().font_body
}

/// The width of the box labelled `label`: its `CommitEdit`, which holds the
/// text input and the padding around it.
fn box_width(ui: &Ui, label: &str) -> f32 {
    let input = Rect::of(&ui.input(label));
    rects(ui, "CommitEdit")
        .into_iter()
        .filter(|rect| rect.holds(&input))
        .map(|rect| rect.w)
        .min_by(f32::total_cmp)
        .unwrap_or_else(|| panic!("no box holds the input labelled {label}"))
}

#[test]
fn every_field_box_has_one_height() {
    let ui = page_ui(EditorCommand::Party, 0, LARGE);
    let boxes: Vec<Rect> = ElementHandle::find_by_element_type_name(&ui.window, "FieldLine")
        .flat_map(|line| line.query_descendants().match_accessible_role(AccessibleRole::TextInput).find_all())
        .map(|input| Rect::of(&input))
        .collect();
    assert!(boxes.len() >= 5, "the character page shows {} boxes", boxes.len());
    let reference = row_height(&ui);
    for rect in &boxes {
        assert!((rect.h - boxes[0].h).abs() <= SLACK, "a box is {} px high, the first is {}", rect.h, boxes[0].h);
    }
    assert!(
        (boxes[0].h - reference).abs() <= 2.0,
        "the boxes are {} px high against the {reference} px row height",
        boxes[0].h
    );
}

#[test]
fn a_number_box_keeps_its_width_at_800() {
    let ui = page_ui(EditorCommand::Party, 0, LARGE);
    let wide = box_width(&ui, "Level");
    resize(&ui, SMALL.0, SMALL.1);
    let narrow = box_width(&ui, "Level");
    assert!((wide - narrow).abs() <= SLACK, "the Level box is {narrow} px at 800 and {wide} px at 1280");
    assert!(
        narrow >= 6.0 * font_body(&ui),
        "the Level box is {narrow} px, six em are {}",
        6.0 * font_body(&ui)
    );

    // Far below the smallest window: the box gives way last, and never below six em.
    resize(&ui, 480.0, 600.0);
    let squeezed = box_width(&ui, "Level");
    assert!(
        squeezed >= 6.0 * font_body(&ui),
        "the Level box is {squeezed} px at 480, six em are {}",
        6.0 * font_body(&ui)
    );
}

#[test]
fn no_two_rows_overlap_on_the_character_page() {
    let ui = page_ui(EditorCommand::Party, 0, LARGE);
    for (width, height) in [LARGE, SMALL] {
        resize(&ui, width, height);
        let list = the_rect(&ui, "FieldList");
        let mut lines = rects(&ui, "FieldLine");
        assert!(lines.len() >= 5, "{width}x{height}: {} rows", lines.len());
        lines.sort_by(|a, b| a.y.total_cmp(&b.y));
        for pair in lines.windows(2) {
            assert!(
                pair[1].y >= pair[0].bottom() - SLACK,
                "{width}x{height}: a row at y {} starts above the row before it ends at {}",
                pair[1].y,
                pair[0].bottom()
            );
        }
        for line in &lines {
            assert!(
                line.x >= list.x - SLACK && line.right() <= list.right() + SLACK,
                "{width}x{height}: a row ({line:?}) leaves the list ({list:?}) horizontally"
            );
        }
    }
}

#[test]
fn more_fields_fit_on_the_character_page() {
    let ui = page_ui(EditorCommand::Party, 0, LARGE);
    for ((width, height), at_least) in [(LARGE, 15), (SMALL, 10)] {
        resize(&ui, width, height);
        let list = the_rect(&ui, "FieldList");
        let shown = rects(&ui, "FieldLine").iter().filter(|line| list.holds(line)).count();
        assert!(shown >= at_least, "{width}x{height}: {shown} rows fit, {at_least} expected");
    }
}

#[test]
fn the_items_rows_fit_at_800() {
    let ui = page_ui(EditorCommand::Items, -1, SMALL);
    let data = section(&ui.bridge_data(), Part::Inventory);
    let all_rows: Vec<_> = lists(&data).iter().flat_map(rows).collect();
    assert!(all_rows.iter().any(|row| row.locked), "the fixture holds a locked row");

    for (width, height) in [SMALL, LARGE] {
        resize(&ui, width, height);
        let page = the_rect(&ui, "InventoryPage");
        let lines: Vec<ElementHandle> = ElementHandle::find_by_element_type_name(&ui.window, "ItemLine").collect();
        assert!(!lines.is_empty());
        let mut whole = 0;
        let mut removes = 0;
        for line in &lines {
            let rect = Rect::of(line);
            assert!(
                rect.x >= page.x - SLACK && rect.right() <= page.right() + SLACK,
                "{width}x{height}: a row ({rect:?}) leaves the page ({page:?}) horizontally"
            );
            for button in line.query_descendants().match_accessible_role(AccessibleRole::Button).find_all() {
                if button.accessible_label().is_some_and(|label| label.starts_with("Remove ")) {
                    removes += 1;
                    let button = Rect::of(&button);
                    assert!(
                        rect.holds(&button) && button.right() <= page.right() + SLACK,
                        "{width}x{height}: a Remove button ({button:?}) leaves its row ({rect:?})"
                    );
                }
            }
            whole += usize::from(page.holds(&rect));
        }
        assert!(removes > 0, "{width}x{height}: some row offers Remove");
        if width > 1000.0 {
            assert!(whole >= 12, "{width}x{height}: {whole} rows fit");
        }
    }
}

#[test]
fn the_magic_buttons_fit_at_800() {
    let ui = page_ui(EditorCommand::Magic, -1, SMALL);
    let page = the_rect(&ui, "CharacterPage");
    let buttons: Vec<Rect> = ElementHandle::find_by_element_type_name(&ui.window, "CharacterPage")
        .flat_map(|page| page.query_descendants().match_accessible_role(AccessibleRole::Button).find_all())
        .map(|button| Rect::of(&button))
        .collect();
    assert!(buttons.len() >= 2, "Magic shows {} character buttons", buttons.len());
    for button in &buttons {
        assert!(page.holds(button), "a button ({button:?}) leaves the page ({page:?})");
    }
    for (index, first) in buttons.iter().enumerate() {
        for second in &buttons[index + 1..] {
            let apart = first.right() <= second.x + SLACK
                || second.right() <= first.x + SLACK
                || first.bottom() <= second.y + SLACK
                || second.bottom() <= first.y + SLACK;
            assert!(apart, "two buttons overlap ({first:?}, {second:?})");
        }
    }
}

#[test]
fn the_picker_button_holds_the_job_name() {
    let ui = page_ui(EditorCommand::Party, 0, SMALL);
    let job = ElementHandle::find_by_element_type_name(&ui.window, "PickerField")
        .flat_map(|picker| picker.query_descendants().match_accessible_role(AccessibleRole::Button).find_all())
        .find(|button| button.accessible_label().is_some_and(|label| label.starts_with("Job: ")))
        .expect("the Job picker");
    let width = Rect::of(&job).w;
    assert!(width >= 10.0 * font_body(&ui), "the Job picker is {width} px wide");
}
