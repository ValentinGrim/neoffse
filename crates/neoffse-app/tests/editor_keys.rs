//! The field controls of the editor, driven by key and pointer events through
//! `Window::dispatch_event` on Slint's testing backend. No window shows and
//! no event loop runs.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Part, Site, Ui, editing_site, field, open_editor};
use i_slint_backend_testing::AccessibleRole;
use neoffse_app::{EditProblem, EditorCommand};
use slint::platform::Key;
use slint::{ComponentHandle, Model};

/// The editor on the character page of the first character.
fn on_character_page(site: &Site) -> Ui {
    let ui = open_editor(site);
    ui.window.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
    ui.navigate(EditorCommand::Party, 0);
    ui
}

fn level_text(ui: &Ui) -> String {
    ui.level().text.to_string()
}

#[test]
fn a_commit_edit_commits_on_enter_once() {
    let site = editing_site();
    let ui = on_character_page(&site);
    assert_eq!(level_text(&ui), "31");

    ui.focus_input("Level");
    ui.replace_text("Level", "40");
    assert_eq!(ui.pending(), 0, "typing sends nothing");
    ui.key(Key::Return);

    assert_eq!(level_text(&ui), "40");
    assert_eq!(ui.pending(), 1);
    ui.key(Key::Tab);
    assert_eq!(ui.pending(), 1, "leaving the box after the commit adds no second entry");
}

#[test]
fn a_commit_edit_commits_on_focus_loss() {
    let site = editing_site();
    let ui = on_character_page(&site);

    ui.focus_input("Level");
    ui.replace_text("Level", "41");
    ui.key(Key::Tab);

    assert_eq!(level_text(&ui), "41");
    assert_eq!(ui.pending(), 1);
    assert_eq!(ui.input_text("Level"), "41");
}

#[test]
fn an_equal_text_commits_nothing() {
    let site = editing_site();
    let ui = on_character_page(&site);

    ui.focus_input("Level");
    ui.key(Key::Return);
    ui.key(Key::Tab);

    assert_eq!(ui.pending(), 0);
    assert!(!ui.bridge().get_has_notice());
}

#[test]
fn escape_reverts_a_pending_text_then_reaches_back() {
    let site = editing_site();
    let ui = on_character_page(&site);

    ui.focus_input("Level");
    ui.key(Key::End);
    ui.type_text("5");
    assert_eq!(ui.input_text("Level"), "315");
    ui.key(Key::Escape);

    assert_eq!(ui.input_text("Level"), "31", "the text is the document's value again");
    assert_eq!(ui.pending(), 0);
    assert_eq!(ui.bridge().get_character(), 0, "the first Escape did not step back");

    ui.key(Key::Escape);

    assert!(ui.on_cards(), "the second Escape steps back");
    assert_eq!(ui.pending(), 0);
}

#[test]
fn a_refused_commit_shows_the_documents_value_again() {
    let site = editing_site();
    let ui = on_character_page(&site);

    ui.focus_input("Level");
    ui.replace_text("Level", "100");
    ui.key(Key::Return);

    let notice = ui.bridge().get_notice();
    assert!(ui.bridge().get_has_notice());
    assert_eq!(notice.problem, EditProblem::AboveCap);
    assert_eq!(notice.label, "Level");
    assert_eq!(ui.input_text("Level"), "31");
    assert_eq!(ui.pending(), 0);
}

fn entry_count(ui: &Ui) -> usize {
    ui.bridge().get_picker_entries().row_count()
}

#[test]
fn the_picker_opens_filters_and_chooses_by_keyboard() {
    let site = editing_site();
    let ui = on_character_page(&site);
    let job = field(&ui.bridge_data(), Part::Party, "Aria", "Job");
    let button = format!("Job: {}", job.shown);

    // Name is the first stop, the job picker the second.
    ui.focus_input("Name");
    ui.key(Key::Tab);
    ui.key(Key::Return);
    let all = entry_count(&ui);
    assert!(all > 4, "the popup lists every job");
    assert!(ui.has_element("Search", AccessibleRole::TextInput), "the popup is open");

    ui.type_text("Mage");
    let narrowed = entry_count(&ui);
    assert!((2..all).contains(&narrowed), "the query narrows the list to {narrowed} of {all}");

    ui.key(Key::DownArrow);
    let second = ui.bridge().get_picker_entries().row_data(1).expect("a second entry");
    ui.key(Key::Return);

    assert!(!ui.has_element("Search", AccessibleRole::TextInput), "choosing closes the popup");
    assert_eq!(ui.pending(), 1);
    let now = field(&ui.bridge_data(), Part::Party, "Aria", "Job");
    assert_eq!(now.text, second.id.to_string());
    assert_ne!(now.text, job.text);
    assert!(!ui.has_element(&button, AccessibleRole::Button), "the button shows the new job");

    // Reopen with a click: Escape closes the popup and nothing else.
    ui.element(&format!("Job: {}", now.shown), AccessibleRole::Button)
        .mock_single_click(slint::platform::PointerEventButton::Left);
    assert!(ui.has_element("Search", AccessibleRole::TextInput));
    ui.key(Key::Escape);
    assert!(!ui.has_element("Search", AccessibleRole::TextInput), "Escape closes the popup");
    assert_eq!(ui.bridge().get_character(), 0, "and does not step back");
    assert_eq!(ui.pending(), 1);
}

#[test]
fn a_blocked_page_ignores_keys() {
    let site = editing_site();
    let ui = on_character_page(&site);
    ui.bridge().invoke_expert_toggle_requested();
    assert!(ui.bridge().get_expert_warning_open());

    ui.focus_input("Level");
    ui.type_text("9");
    ui.key(Key::Return);
    ui.key(Key::Tab);

    assert_eq!(ui.input_text("Level"), "31");
    assert_eq!(ui.pending(), 0);
    assert!(!ui.bridge().get_has_notice());
}
