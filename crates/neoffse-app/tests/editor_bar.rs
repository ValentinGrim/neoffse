//! The pending changes bar: its count, its buttons and its visibility. The
//! testing backend draws nothing, so a test reads the bridge and the
//! accessibility tree.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Ui, editing_site, open_editor};
use i_slint_backend_testing::AccessibleRole::{Button, Text};
use neoffse_app::EditProblem;

fn bar_shown(ui: &Ui) -> bool {
    ui.has_element("Review & save", Button)
}

fn gil_text(ui: &Ui) -> String {
    ui.gil().text.to_string()
}

#[test]
fn a_clean_save_shows_no_bar() {
    let site = editing_site();
    let ui = open_editor(&site);
    assert_eq!(ui.bridge().get_pending_count(), 0);
    assert!(!bar_shown(&ui));
    assert!(!ui.has_element("Undo", Button));
    assert!(!ui.has_element("Redo", Button));
}

#[test]
fn an_edit_shows_the_bar_with_its_count() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();

    assert!(bar_shown(&ui));
    assert_eq!(ui.bridge().get_pending_count(), 1);
    assert_eq!(ui.element("Undo", Button).accessible_enabled(), Some(true));
    assert_eq!(ui.element("Redo", Button).accessible_enabled(), Some(false));
    assert_eq!(ui.element("Review & save", Button).accessible_enabled(), Some(true));

    ui.commit(ui.gil().key, "600");
    assert_eq!(ui.bridge().get_pending_count(), 2);
}

#[test]
fn the_undo_button_undoes_and_hides_the_bar() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();

    ui.element("Undo", Button).invoke_accessible_default_action();

    assert_eq!(ui.pending(), 0);
    assert_eq!(gil_text(&ui), "123456");
    assert!(!bar_shown(&ui));
    assert!(ui.bridge().get_can_redo());
    assert!(!ui.bridge().get_can_undo());
}

#[test]
fn the_redo_button_redoes() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.bridge().invoke_undo_requested();
    assert!(!bar_shown(&ui));

    ui.bridge().invoke_redo_requested();
    assert_eq!(ui.pending(), 1);
    assert_eq!(gil_text(&ui), "500");
    assert!(bar_shown(&ui));
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
    assert!(bar_shown(&ui), "the notice and the bar coexist");
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
    assert!(bar_shown(&ui));
}

#[test]
fn a_refused_edit_adds_no_entry() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.level().key, "100");

    assert!(ui.bridge().get_has_notice());
    assert_eq!(ui.bridge().get_notice().problem, EditProblem::AboveCap);
    assert_eq!(ui.pending(), 0);
    assert_eq!(ui.bridge().get_pending_count(), 0);
    assert!(!bar_shown(&ui));
}

#[test]
fn a_dialog_greys_the_bar() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.make_dirty();
    ui.window.invoke_back_requested();
    assert!(ui.bridge().get_discard_dialog_open());

    for label in ["Undo", "Redo", "Review & save"] {
        assert_eq!(ui.element(label, Button).accessible_enabled(), Some(false), "{label}");
    }
}
