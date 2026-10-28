//! The character page reads its card through the table Rust sends with the
//! overview, not through its own position. `card_of_page` is pure; the page
//! tests build the bridge state by hand on the testing backend.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Ui, editing, editing_site, items, open_editor};
use i_slint_backend_testing::AccessibleRole::Text;
use neoffse_app::editor::card_of_page;
use neoffse_app::{CharacterCard, EditorCommand, EditorData, EntityPage, Overview, SectionData};
use slint::{ModelRc, SharedString, VecModel};

fn card(page: i32, name: &str, job: &str) -> CharacterCard {
    CharacterCard {
        page,
        name: name.into(),
        job: job.into(),
        ..CharacterCard::default()
    }
}

fn pages_of(cards: &[i32]) -> Vec<CharacterCard> {
    cards.iter().map(|page| card(*page, "", "")).collect()
}

#[test]
fn card_of_page_cases() {
    assert_eq!(card_of_page(&pages_of(&[0, 1, 2, 3]), 4), [0, 1, 2, 3]);
    assert_eq!(card_of_page(&pages_of(&[1, 0]), 2), [1, 0], "reordered");
    assert_eq!(card_of_page(&pages_of(&[1]), 2), [-1, 0], "a page with no card");
    assert_eq!(card_of_page(&pages_of(&[5]), 2), [-1, -1], "a page past the table");
    assert_eq!(card_of_page(&pages_of(&[-1]), 2), [-1, -1], "a negative page");
    assert_eq!(card_of_page(&pages_of(&[0, 0]), 1), [0], "the first card wins");
    assert_eq!(card_of_page(&[], 3), [-1, -1, -1]);
    assert_eq!(card_of_page(&pages_of(&[0]), 0), Vec::<i32>::new());
}

#[test]
fn the_ff1_overview_maps_every_page_to_its_card() {
    let site = editing_site();
    let ed = editing(&site);
    assert_eq!(items(&ed.editor().overview().card_of_page), [0, 1, 2, 3]);
}

fn model<T: Clone + 'static>(rows: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(rows))
}

/// The editor on page `page` of two pages "First" and "Second", with the
/// given cards and table.
fn on_page(cards: Vec<CharacterCard>, table: Vec<i32>, page: i32) -> (common::Site, Ui) {
    let site = editing_site();
    let ui = open_editor(&site);
    let page_named = |label: &str| EntityPage {
        label: SharedString::from(label),
        ..EntityPage::default()
    };
    let party = SectionData {
        present: true,
        pages: model(vec![page_named("First"), page_named("Second")]),
        ..SectionData::default()
    };
    ui.bridge().set_data(EditorData {
        party,
        ..EditorData::default()
    });
    ui.bridge().set_overview(Overview {
        present: true,
        cards: model(cards),
        card_of_page: model(table),
        ..Overview::default()
    });
    ui.navigate(EditorCommand::Party, page);
    (site, ui)
}

#[test]
fn the_page_shows_the_card_that_opens_it() {
    let (_site, ui) = on_page(vec![card(1, "Mo", "Monk"), card(0, "Fi", "Fighter")], vec![1, 0], 0);
    assert!(ui.has_element("First", Text));
    assert!(ui.has_element("Fighter", Text));
    assert!(!ui.has_element("Monk", Text));
}

#[test]
fn a_page_without_a_card_shows_no_job() {
    let (_site, ui) = on_page(vec![card(1, "Mo", "Monk")], vec![-1, 0], 0);
    assert!(ui.has_element("First", Text));
    assert!(!ui.has_element("Monk", Text));
}

#[test]
fn a_short_table_does_not_fall_back_to_the_first_card() {
    // An entry missing from the table reads as 0 in Slint: the first card.
    // The second card is at the page's own position, which the old rule read.
    let (_site, ui) = on_page(vec![card(0, "Fi", "Fighter"), card(1, "Mo", "Monk")], vec![0], 1);
    assert!(ui.has_element("Second", Text));
    assert!(!ui.has_element("Fighter", Text));
    assert!(!ui.has_element("Monk", Text));
}

#[test]
fn an_empty_table_shows_no_card_data() {
    let (_site, ui) = on_page(Vec::new(), Vec::new(), 0);
    assert!(ui.has_element("First", Text));
    assert!(!ui.has_element("Fighter", Text));
}
