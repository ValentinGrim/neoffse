//! The pending changes bar: its count, its buttons and its idle state. The
//! bar shows at all times; at zero it says so and Review & save waits. The
//! testing backend draws nothing, so a test reads the bridge and the
//! accessibility tree.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Rect, SLACK, Ui, editing_site, first_rect, open_editor, resize};
use i_slint_backend_testing::AccessibleRole::{Button, Text};
use i_slint_backend_testing::ElementHandle;
use neoffse_app::EditProblem;
use slint::platform::Key;

fn review_enabled(ui: &Ui) -> bool {
    ui.element("Review & save", Button).accessible_enabled() == Some(true)
}

fn enabled(ui: &Ui, label: &str) -> bool {
    ui.element(label, Button).accessible_enabled() == Some(true)
}

fn button_rect(ui: &Ui, label: &str) -> Rect {
    Rect::of(&ui.element(label, Button))
}

fn gil_text(ui: &Ui) -> String {
    ui.gil().text.to_string()
}

#[test]
fn a_clean_save_shows_the_idle_bar() {
    let site = editing_site();
    let ui = open_editor(&site);
    assert_eq!(ui.bridge().get_pending_count(), 0);
    assert!(ElementHandle::find_by_element_type_name(&ui.window, "PendingBar").next().is_some());
    assert!(ui.has_element("No unsaved changes", Text));
    for label in ["Undo", "Redo", "Review & save"] {
        assert_eq!(ui.element(label, Button).accessible_enabled(), Some(false), "{label}");
    }
}

#[test]
fn an_edit_enables_review_and_undo() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();

    assert!(review_enabled(&ui));
    assert_eq!(ui.bridge().get_pending_count(), 1);
    assert_eq!(ui.element("Undo", Button).accessible_enabled(), Some(true));
    assert_eq!(ui.element("Redo", Button).accessible_enabled(), Some(false));
    assert_eq!(ui.element("Review & save", Button).accessible_enabled(), Some(true));

    ui.commit(ui.gil().key, "600");
    assert_eq!(ui.bridge().get_pending_count(), 2);
}

#[test]
fn the_undo_button_undoes_and_the_bar_idles() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();

    ui.element("Undo", Button).invoke_accessible_default_action();

    assert_eq!(ui.pending(), 0);
    assert_eq!(gil_text(&ui), "123456");
    assert!(!review_enabled(&ui));
    assert!(ui.has_element("No unsaved changes", Text));
    assert!(!enabled(&ui, "Undo"));
    assert!(enabled(&ui, "Redo"));
    assert!(ui.bridge().get_can_redo());
    assert!(!ui.bridge().get_can_undo());
}

#[test]
fn the_redo_button_redoes() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.bridge().invoke_undo_requested();
    assert!(!review_enabled(&ui));

    ui.bridge().invoke_redo_requested();
    assert_eq!(ui.pending(), 1);
    assert_eq!(gil_text(&ui), "500");
    assert!(review_enabled(&ui));
}

#[test]
fn the_redo_button_redoes_while_the_bar_stays() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.commit(ui.gil().key, "600");
    ui.element("Undo", Button).invoke_accessible_default_action();

    assert_eq!(ui.pending(), 1);
    assert_eq!(ui.element("Redo", Button).accessible_enabled(), Some(true));
    ui.element("Redo", Button).invoke_accessible_default_action();
    assert_eq!(ui.pending(), 2);
    assert_eq!(gil_text(&ui), "600");
}

#[test]
fn the_count_text_is_singular_then_plural() {
    let site = editing_site();
    let ui = open_editor(&site);
    assert!(ui.has_element("No unsaved changes", Text), "zero");
    ui.make_dirty();
    assert!(ui.has_element("1 unsaved change", Text), "singular");
    ui.commit(ui.gil().key, "600");
    assert!(ui.has_element("2 unsaved changes", Text), "plural");
}

#[test]
fn an_undo_clears_the_level_up_notice() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.level().key, "34");

    assert!(ui.bridge().get_has_notice());
    assert_eq!(ui.bridge().get_notice().problem, EditProblem::StatsRaised);
    assert!(review_enabled(&ui), "the notice and the bar coexist");
    assert_eq!(ui.level().text, "34");

    ui.element("Undo", Button).invoke_accessible_default_action();

    assert!(!ui.bridge().get_has_notice());
    assert_eq!(ui.level().text, "31");
    assert_eq!(ui.pending(), 0);
}

#[test]
fn a_redo_does_not_claim_a_new_draw() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.level().key, "34");
    ui.bridge().invoke_undo_requested();
    assert_eq!(ui.level().text, "31");

    ui.bridge().invoke_redo_requested();

    assert_eq!(ui.level().text, "34");
    assert!(!ui.bridge().get_has_notice());
    assert!(review_enabled(&ui));
}

#[test]
fn a_refused_edit_adds_no_entry() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.gil().key, "1000000");

    assert!(ui.bridge().get_has_notice());
    assert_eq!(ui.bridge().get_notice().problem, EditProblem::AboveCap);
    assert_eq!(ui.pending(), 0);
    assert_eq!(ui.bridge().get_pending_count(), 0);
    assert!(!review_enabled(&ui));
    assert!(ui.has_element("No unsaved changes", Text));
}

#[test]
fn a_dialog_greys_the_bar() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.ask_discard();

    for label in ["Undo", "Redo", "Review & save"] {
        assert_eq!(ui.element(label, Button).accessible_enabled(), Some(false), "{label}");
    }
}

#[test]
fn redo_is_reachable_after_undoing_every_edit() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.element("Undo", Button).invoke_accessible_default_action();

    assert_eq!(ui.pending(), 0);
    assert!(ui.has_element("No unsaved changes", Text), "the bar stays");
    assert!(enabled(&ui, "Redo"));

    ui.element("Redo", Button).invoke_accessible_default_action();

    assert_eq!(ui.pending(), 1);
    assert_eq!(gil_text(&ui), "500");
    assert!(ui.has_element("1 unsaved change", Text));
    assert!(review_enabled(&ui));
}

#[test]
fn review_and_save_does_nothing_at_zero() {
    let site = editing_site();
    let ui = open_editor(&site);
    assert!(!review_enabled(&ui));

    ui.bridge().invoke_review_requested();
    assert!(!ui.bridge().get_review_open());
    ui.chord(&[Key::Control], "s");
    assert!(!ui.bridge().get_review_open());
}

#[test]
fn working_keeps_its_word_at_zero() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.bridge().set_working(true);

    assert!(ui.has_element("Working...", Text));
    assert!(!ui.has_element("No unsaved changes", Text));
    for label in ["Undo", "Redo", "Review & save"] {
        assert!(!enabled(&ui, label), "{label}");
    }

    ui.bridge().set_working(false);
    assert!(ui.has_element("No unsaved changes", Text));
}

#[test]
fn the_bar_keeps_its_place_when_the_first_edit_lands() {
    let site = editing_site();
    let ui = open_editor(&site);
    resize(&ui, 1280.0, 800.0);
    let (bar, cards) = (first_rect(&ui, "PendingBar"), first_rect(&ui, "PartyCards"));

    ui.make_dirty();

    for (what, before, after) in [
        ("the bar", bar, first_rect(&ui, "PendingBar")),
        ("the cards", cards, first_rect(&ui, "PartyCards")),
    ] {
        let moved = [before.x - after.x, before.y - after.y, before.w - after.w, before.h - after.h];
        assert!(moved.iter().all(|delta| delta.abs() <= SLACK), "{what} moved: {before:?} then {after:?}");
    }
}

#[test]
fn the_idle_bar_fits_at_800() {
    let site = editing_site();
    let ui = open_editor(&site);
    resize(&ui, 800.0, 600.0);
    let window = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    let bar = first_rect(&ui, "PendingBar");
    let labels = ["Undo", "Redo", "Review & save"];
    let buttons: Vec<Rect> = labels.into_iter().map(|label| button_rect(&ui, label)).collect();
    for (label, button) in labels.into_iter().zip(&buttons) {
        assert!(window.holds(button), "{label} ({button:?}) leaves the window");
        assert!(bar.holds(button), "{label} ({button:?}) leaves the bar ({bar:?})");
    }
    for pair in buttons.windows(2) {
        assert!(pair[0].right() <= pair[1].x + SLACK, "two bar buttons overlap ({:?}, {:?})", pair[0], pair[1]);
    }
    let text = Rect::of(&ui.element("No unsaved changes", Text));
    assert!(
        text.right() <= buttons[0].x + SLACK,
        "the sentence ({text:?}) runs under Undo ({:?})",
        buttons[0]
    );
}
