//! The level-up notice rule of the view model: when an action raised levels
//! in default mode, `with_growth` returns the notice itself. No window runs.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Editing, Part, editing, editing_site, field, saving};
use neoffse::Running;
use neoffse_app::editor::{Input, growth_notice, problem_notice};
use neoffse_app::{EditNotice, EditProblem};

/// Commits `text` to Aria's `label` field through `with_growth`. The key comes
/// from a fresh `data()` call: keys last until the next one.
fn commit(ed: &Editing, label: &str, text: &str) -> Option<EditNotice> {
    let key = field(&ed.data(), Part::Party, "Aria", label).key;
    ed.editor().with_growth(|| ed.editor().commit(key, Input::Text(text.to_owned())))
}

fn gil(ed: &Editing, text: &str) -> Option<EditNotice> {
    let key = field(&ed.data(), Part::Inventory, "Gil", "Gil").key;
    ed.editor().with_growth(|| ed.editor().commit(key, Input::Text(text.to_owned())))
}

fn level(ed: &Editing) -> String {
    field(&ed.data(), Part::Party, "Aria", "Level").text.to_string()
}

#[test]
fn a_level_commit_returns_the_growth_notice() {
    let site = editing_site();
    let ed = editing(&site);
    assert_eq!(commit(&ed, "Level", "34"), Some(growth_notice(3)));
}

#[test]
fn one_level_returns_a_count_of_one() {
    let site = editing_site();
    let ed = editing(&site);
    assert_eq!(commit(&ed, "Level", "32"), Some(growth_notice(1)));
}

#[test]
fn an_experience_commit_that_raises_the_level_returns_it() {
    let site = editing_site();
    let ed = editing(&site);
    assert_eq!(commit(&ed, "Experience", "130000"), Some(growth_notice(3)));
}

#[test]
fn the_before_state_is_the_session_not_a_window() {
    let site = editing_site();
    let ed = editing(&site);
    // A commit outside the wrapper moves the level from 31 to 34 first.
    let key = field(&ed.data(), Part::Party, "Aria", "Level").key;
    assert!(ed.editor().commit(key, Input::Text("34".into())).is_none());
    assert_eq!(commit(&ed, "Level", "35"), Some(growth_notice(1)));
}

#[test]
fn no_notice_without_a_rise() {
    let site = editing_site();
    let ed = editing(&site);
    // A fall last: a fall first would turn the next level edit into a rise.
    assert_eq!(commit(&ed, "Level", "31"), None, "the same level");
    assert_eq!(commit(&ed, "Experience", "100200"), None, "inside the level");
    assert_eq!(commit(&ed, "Level", "1"), None, "a fall");
    assert_eq!(gil(&ed, "500"), None, "gil");
}

#[test]
fn an_error_wins() {
    let site = editing_site();
    let ed = editing(&site);
    assert_eq!(commit(&ed, "Level", "many"), Some(problem_notice(EditProblem::NotANumber, "Level")));
    let over = gil(&ed, "1000000").expect("a notice");
    assert_eq!(over.problem, EditProblem::AboveCap);
}

#[test]
fn expert_mode_returns_none_and_still_edits() {
    let site = editing_site();
    let ed = editing(&site);
    ed.editor().set_expert(true);
    assert_eq!(commit(&ed, "Level", "34"), None);
    assert_eq!(level(&ed), "34");
}

#[test]
fn no_save_open_returns_not_editable() {
    let site = editing_site();
    let ed = editing(&site);
    let key = field(&ed.data(), Part::Party, "Aria", "Level").key;
    ed.editor().back();
    let notice = ed.editor().with_growth(|| ed.editor().commit(key, Input::Text("34".into())));
    assert_eq!(notice.expect("a notice").problem, EditProblem::NotEditable);
}

#[test]
fn a_running_save_returns_not_editable() {
    let site = editing_site();
    let saving = saving(&site);
    let ed = &saving.ed;
    let key = field(&ed.data(), Part::Party, "Aria", "Level").key;
    saving.save(Running::Allow);
    assert!(ed.editor().is_saving());
    let notice = ed.editor().with_growth(|| ed.editor().commit(key, Input::Text("34".into())));
    assert_eq!(notice.expect("a notice").problem, EditProblem::NotEditable);
}
