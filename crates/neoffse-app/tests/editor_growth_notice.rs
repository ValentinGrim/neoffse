//! The info notice after a level increase: `levels_gained`, `growth_notice`
//! and the glue that shows them. The testing backend draws nothing on screen.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Part, Site, Ui, editing_site, field, open_editor};
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use neoffse_app::editor::{growth_notice, levels_gained};
use neoffse_app::{CharacterCard, EditProblem, Overview};
use slint::{ModelRc, VecModel};

fn overview_of(present: bool, cards: &[(i32, &str)]) -> Overview {
    let cards: Vec<CharacterCard> = cards
        .iter()
        .map(|(page, level)| CharacterCard {
            page: *page,
            level: (*level).into(),
            ..CharacterCard::default()
        })
        .collect();
    Overview {
        present,
        cards: ModelRc::new(VecModel::from(cards)),
        ..Overview::default()
    }
}

fn gained(before: &[(i32, &str)], after: &[(i32, &str)]) -> u32 {
    levels_gained(&overview_of(true, before), &overview_of(true, after))
}

#[test]
fn levels_gained_cases() {
    assert_eq!(gained(&[(0, "31")], &[(0, "34")]), 3);
    assert_eq!(gained(&[(0, "31"), (1, "32")], &[(0, "33"), (1, "34")]), 4);
    assert_eq!(gained(&[(0, "31")], &[(0, "31")]), 0, "equal level");
    assert_eq!(gained(&[(0, "34")], &[(0, "31")]), 0, "a fall");
    assert_eq!(gained(&[(0, "")], &[(0, "34")]), 0, "no level before");
    assert_eq!(gained(&[(0, "31")], &[(0, "")]), 0, "no level after");
    assert_eq!(gained(&[(0, "many")], &[(0, "34")]), 0, "no number before");
    assert_eq!(gained(&[(0, "31")], &[(0, "many")]), 0, "no number after");
    assert_eq!(gained(&[(0, "31")], &[(1, "34")]), 0, "another page");
    assert_eq!(gained(&[(0, "31")], &[(0, "34"), (1, "40")]), 3, "a card on one side only");
    assert_eq!(gained(&[(0, "31"), (1, "40")], &[(0, "34")]), 3, "a card on one side only");
    assert_eq!(gained(&[], &[]), 0);
    assert_eq!(levels_gained(&overview_of(false, &[(0, "31")]), &overview_of(true, &[(0, "34")])), 0);
    assert_eq!(levels_gained(&overview_of(true, &[(0, "31")]), &overview_of(false, &[(0, "34")])), 0);
    assert_eq!(levels_gained(&Overview::default(), &Overview::default()), 0);
}

#[test]
fn levels_gained_near_the_top_does_not_overflow() {
    let top = u32::MAX.to_string();
    let before = [(0, "0"), (1, "0"), (2, "0")];
    let after = [(0, top.as_str()), (1, top.as_str()), (2, "5")];
    assert_eq!(gained(&before, &after), u32::MAX);
    assert_eq!(gained(&[(0, "-9223372036854775808")], &[(0, "9223372036854775807")]), u32::MAX);
}

#[test]
fn growth_notice_carries_only_the_count() {
    let notice = growth_notice(3);
    assert_eq!(notice.problem, EditProblem::StatsRaised);
    assert_eq!(notice.value, "3");
    for text in [&notice.label, &notice.min, &notice.max, &notice.cap] {
        assert_eq!(text, "");
    }
}

fn text_of(ui: &Ui, label: &str) -> String {
    field(&ui.bridge_data(), Part::Party, "Aria", label).text.to_string()
}

/// Commits `text` to Aria's `label` field.
fn commit(ui: &Ui, label: &str, text: &str) {
    let key = field(&ui.bridge_data(), Part::Party, "Aria", label).key;
    ui.commit(key, text);
}

fn opened(site: &Site) -> Ui {
    open_editor(site)
}

fn expect_raised(ui: &Ui, count: &str) {
    let bridge = ui.bridge();
    assert!(bridge.get_has_notice());
    assert_eq!(bridge.get_notice().problem, EditProblem::StatsRaised);
    assert_eq!(bridge.get_notice().value, count);
    assert!(!bridge.get_offer_expert());
}

#[test]
fn a_level_commit_shows_the_info_notice() {
    let site = editing_site();
    let ui = opened(&site);
    commit(&ui, "Level", "34");
    expect_raised(&ui, "3");
    assert_eq!(text_of(&ui, "Level"), "34");
    assert_eq!(text_of(&ui, "Accuracy gain"), "155");
    assert_eq!(text_of(&ui, "Magic evasion gain"), "142");
    assert!(text_of(&ui, "Max HP gain").parse::<i64>().unwrap() >= 461);
    assert_eq!(ui.pending(), 1);
}

#[test]
fn an_experience_commit_that_raises_the_level_shows_it() {
    let site = editing_site();
    let ui = opened(&site);
    commit(&ui, "Experience", "130000");
    expect_raised(&ui, "3");
    assert_eq!(text_of(&ui, "Level"), "34");
    assert_eq!(text_of(&ui, "Accuracy gain"), "155");
}

#[test]
fn one_level_reads_as_one() {
    let site = editing_site();
    let ui = opened(&site);
    commit(&ui, "Level", "32");
    expect_raised(&ui, "1");
}

#[test]
fn no_notice_without_a_rise() {
    let site = editing_site();
    let ui = opened(&site);
    // The same level, an experience inside the level, a fall last: a fall
    // first would turn the next level edit into a rise.
    for (label, text) in [("Experience", "100200"), ("Level", "31"), ("Level", "1")] {
        commit(&ui, label, text);
        assert!(!ui.bridge().get_has_notice(), "{label} {text}");
    }
    ui.commit(ui.gil().key, "500");
    assert!(!ui.bridge().get_has_notice(), "gil");
}

#[test]
fn expert_mode_shows_no_notice_and_grows_nothing() {
    let site = editing_site();
    let ui = opened(&site);
    ui.bridge().invoke_expert_toggle_requested();
    ui.bridge().invoke_expert_warning_confirmed();
    assert!(ui.app.editor.expert());
    commit(&ui, "Level", "34");
    assert!(!ui.bridge().get_has_notice());
    assert_eq!(text_of(&ui, "Level"), "34");
    assert_eq!(text_of(&ui, "Accuracy gain"), "143");
}

#[test]
fn an_error_notice_wins() {
    let site = editing_site();
    let ui = opened(&site);
    let gil = ui.gil();
    ui.commit(gil.key, "1000000");
    assert!(ui.bridge().get_has_notice());
    assert_eq!(ui.bridge().get_notice().problem, EditProblem::AboveCap);
    assert_eq!(ui.gil().text, gil.text);
}

#[test]
fn the_next_action_clears_the_notice() {
    let site = editing_site();
    let ui = opened(&site);
    commit(&ui, "Level", "34");
    assert!(ui.bridge().get_has_notice());
    ui.commit(ui.gil().key, "500");
    assert!(!ui.bridge().get_has_notice());
}

fn sentence_shown(ui: &Ui, sentence: &str) -> bool {
    ElementHandle::find_by_accessible_label(&ui.window, sentence).any(|element| element.accessible_role() == Some(AccessibleRole::Text))
}

#[test]
fn the_sentence_is_drawn() {
    let site = editing_site();
    let ui = opened(&site);
    commit(&ui, "Level", "34");
    assert!(sentence_shown(
        &ui,
        "Stats raised for 3 levels with a random draw, like the game. Undo to cancel."
    ));
    commit(&ui, "Level", "35");
    assert!(sentence_shown(
        &ui,
        "Stats raised for 1 level with a random draw, like the game. Undo to cancel."
    ));
}
