//! The Toggle and Choice controls show the document's value at every moment:
//! after their own action, after a refusal, and after undo and redo. The
//! window shows one `FieldEditor` on Slint's testing backend. No window opens
//! and no event loop runs; `settle` stands for the change handlers the loop
//! runs once per event.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod field_harness;

use std::cell::RefCell;
use std::rc::Rc;

use field_harness::{
    EditorBridge, FieldKind, FieldRow, Harness, checked, choice_document, choice_row, combo_width, harness, key, names, settle, shown, toggle_document,
    toggle_row, typed,
};
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, SharedString};

fn space(ui: &Harness) {
    key(ui, " ");
    settle();
}

fn tab(ui: &Harness) {
    key(ui, Key::Tab);
    settle();
}

fn down(ui: &Harness) {
    key(ui, Key::DownArrow);
    settle();
}

fn name(shown: Option<SharedString>) -> String {
    shown.map(|text| text.to_string()).unwrap_or_default()
}

#[test]
fn a_toggle_commits_on_space() {
    i_slint_backend_testing::init_no_event_loop();
    let ui = Harness::new().unwrap();
    ui.set_data(FieldRow {
        kind: FieldKind::Toggle,
        editable: true,
        ..FieldRow::default()
    });
    let sent = Rc::new(RefCell::new(Vec::new()));
    let record = Rc::clone(&sent);
    ui.global::<EditorBridge>()
        .on_field_toggled(move |_key, checked| record.borrow_mut().push(checked));

    ui.window().dispatch_event(WindowEvent::KeyPressed { text: Key::Tab.into() });
    ui.window().dispatch_event(WindowEvent::KeyReleased { text: Key::Tab.into() });
    for text in [" ", " "] {
        ui.window().dispatch_event(WindowEvent::KeyPressed { text: text.into() });
        ui.window().dispatch_event(WindowEvent::KeyReleased { text: text.into() });
    }

    assert_eq!(
        *sent.borrow(),
        [true, true],
        "one toggle per Space, and the box shows the document value again after each"
    );
}

#[test]
fn a_toggle_follows_the_document_after_its_own_toggle() {
    let ui = harness();
    let _document = toggle_document(&ui, false);
    assert_eq!(checked(&ui), Some(false));

    tab(&ui);
    space(&ui);
    assert_eq!(checked(&ui), Some(true), "the document accepted the toggle");

    ui.set_data(toggle_row(false));
    assert_eq!(checked(&ui), Some(false), "the row went back to false");
    ui.set_data(toggle_row(true));
    assert_eq!(checked(&ui), Some(true));
    ui.set_data(toggle_row(false));
    assert_eq!(checked(&ui), Some(false), "the box still follows the row");
}

#[test]
fn a_toggle_follows_undo_and_redo() {
    let ui = harness();
    let document = toggle_document(&ui, false);

    tab(&ui);
    space(&ui);
    assert_eq!(checked(&ui), Some(true), "edit");
    document.undo();
    assert_eq!(checked(&ui), Some(false), "undo");
    document.redo();
    assert_eq!(checked(&ui), Some(true), "redo");
    document.undo();
    assert_eq!(checked(&ui), Some(false), "undo again");
}

#[test]
fn a_refused_toggle_shows_the_documents_value() {
    let ui = harness();
    ui.set_data(toggle_row(false));

    tab(&ui);
    space(&ui);
    assert_eq!(checked(&ui), Some(false), "the refusal leaves the document's value");

    ui.set_data(toggle_row(true));
    assert_eq!(checked(&ui), Some(true), "the box follows the row after a refusal");
}

#[test]
fn a_choice_follows_the_document_after_its_own_pick() {
    let ui = harness();
    let _document = choice_document(&ui, 0);
    assert_eq!(name(shown(&ui)), "Alpha");

    tab(&ui);
    down(&ui);
    assert_eq!(name(shown(&ui)), "Beta", "the document accepted the pick");

    ui.set_data(choice_row(0, &names()));
    assert_eq!(name(shown(&ui)), "Alpha");
    ui.set_data(choice_row(2, &names()));
    assert_eq!(name(shown(&ui)), "Gamma");
    ui.set_data(choice_row(0, &names()));
    assert_eq!(name(shown(&ui)), "Alpha", "the drop-down still follows the row");
}

#[test]
fn a_choice_follows_undo_and_redo() {
    let ui = harness();
    let document = choice_document(&ui, 0);

    tab(&ui);
    down(&ui);
    assert_eq!(name(shown(&ui)), "Beta", "first pick");
    down(&ui);
    assert_eq!(name(shown(&ui)), "Gamma", "second pick");
    document.undo();
    assert_eq!(name(shown(&ui)), "Beta", "undo");
    document.undo();
    assert_eq!(name(shown(&ui)), "Alpha", "undo again");
    document.redo();
    assert_eq!(name(shown(&ui)), "Beta", "redo");
    document.redo();
    assert_eq!(name(shown(&ui)), "Gamma", "redo again");
}

#[test]
fn a_refused_choice_shows_the_documents_value() {
    let ui = harness();
    ui.set_data(choice_row(0, &names()));

    tab(&ui);
    down(&ui);
    assert_eq!(name(shown(&ui)), "Alpha", "the refusal leaves the document's value");

    ui.set_data(choice_row(2, &names()));
    assert_eq!(name(shown(&ui)), "Gamma", "the drop-down follows the row after a refusal");
}

#[test]
fn a_value_outside_the_names_shows_no_name() {
    let ui = harness();
    ui.set_data(choice_row(1, &names()));
    assert_eq!(name(shown(&ui)), "Beta");

    ui.set_data(FieldRow {
        text: "7".into(),
        ..choice_row(-1, &names())
    });
    let now = name(shown(&ui));
    assert!(!names().contains(&now.as_str()), "a value no name covers shows no name, not {now:?}");
    assert_eq!(typed(&ui).as_deref(), Some("7"), "the number box holds the raw value");

    ui.set_data(choice_row(2, &names()));
    assert_eq!(name(shown(&ui)), "Gamma");
}

#[test]
fn a_choice_keeps_its_width() {
    let ui = harness();
    ui.set_data(choice_row(0, &names()));

    let width = combo_width(&ui);
    assert!(width >= 160.0, "the drop-down is {width} px wide");
}
