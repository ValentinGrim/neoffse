//! The history of the open session: counts, undo, redo and the review rows.
//! No window.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::sync::{Arc, Mutex, PoisonError};

use common::{Editing, Part, editing, editing_site, field};
use neoffse_app::dispatch::Dispatcher;
use neoffse_app::editor::{History, Input};
use neoffse_app::{EditProblem, ReviewKind, ReviewRow};

fn text(value: &str) -> Input {
    Input::Text(value.to_owned())
}

fn gil_text(ed: &Editing) -> String {
    field(&ed.data(), Part::Inventory, "Gil", "Gil").text.to_string()
}

fn set_gil(ed: &Editing, value: &str) {
    let key = field(&ed.data(), Part::Inventory, "Gil", "Gil").key;
    assert!(ed.editor().commit(key, text(value)).is_none());
}

fn row(kind: ReviewKind, entity: &str, label: &str, before: &str, after: &str) -> ReviewRow {
    ReviewRow {
        kind,
        entity: entity.into(),
        label: label.into(),
        before: before.into(),
        after: after.into(),
    }
}

fn heading(entity: &str) -> ReviewRow {
    row(ReviewKind::Heading, entity, "", "", "")
}

fn find<'a>(rows: &'a [ReviewRow], label: &str) -> &'a ReviewRow {
    rows.iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| panic!("no review row labelled {label}"))
}

#[test]
fn a_clean_save_has_no_history() {
    let site = editing_site();
    let ed = editing(&site);
    let history = ed.editor().history();
    assert_eq!(history.pending, 0);
    assert!(!history.can_undo);
    assert!(!history.can_redo);
}

#[test]
fn a_commit_makes_one_entry() {
    let site = editing_site();
    let ed = editing(&site);
    set_gil(&ed, "500");
    let history = ed.editor().history();
    assert_eq!(history.pending, 1);
    assert!(history.can_undo);
    assert!(!history.can_redo);

    assert!(ed.editor().undo().is_none());
    let history = ed.editor().history();
    assert_eq!(history.pending, 0);
    assert!(history.can_redo);

    assert!(ed.editor().redo().is_none());
    assert_eq!(ed.editor().history().pending, 1);
}

#[test]
fn pending_is_the_distance_to_the_clean_point() {
    let site = editing_site();
    let ed = editing(&site);
    set_gil(&ed, "500");
    set_gil(&ed, "600");
    assert_eq!(ed.editor().history().pending, 2);
    assert!(ed.editor().undo().is_none());
    assert_eq!(ed.editor().history().pending, 1);
}

#[test]
fn undo_and_redo_report_nothing_when_empty() {
    let site = editing_site();
    let ed = editing(&site);
    assert!(ed.editor().undo().is_none());
    assert!(ed.editor().redo().is_none());
    assert_eq!(ed.editor().history(), History::default());
}

#[test]
fn undo_restores_the_data() {
    let site = editing_site();
    let ed = editing(&site);
    set_gil(&ed, "500");
    assert_eq!(gil_text(&ed), "500");
    assert!(ed.editor().undo().is_none());
    assert_eq!(gil_text(&ed), "123456");
}

#[test]
fn undo_and_redo_work_in_expert_mode() {
    let site = editing_site();
    let ed = editing(&site);
    ed.editor().set_expert(true);
    set_gil(&ed, "500");
    assert!(ed.editor().undo().is_none());
    assert_eq!(gil_text(&ed), "123456");
    assert!(ed.editor().redo().is_none());
    assert_eq!(gil_text(&ed), "500");
}

#[test]
fn review_lists_the_net_difference() {
    let site = editing_site();
    let ed = editing(&site);
    set_gil(&ed, "500");
    assert_eq!(
        ed.editor().review().unwrap(),
        [heading("Gil"), row(ReviewKind::Field, "Gil", "Gil", "123456", "500")]
    );
}

#[test]
fn a_value_set_back_reviews_as_empty() {
    let site = editing_site();
    let ed = editing(&site);
    set_gil(&ed, "500");
    set_gil(&ed, "123456");
    assert_eq!(ed.editor().history().pending, 2);
    assert_eq!(ed.editor().review().unwrap().len(), 0);
}

#[test]
fn a_clean_session_reviews_as_empty() {
    let site = editing_site();
    let ed = editing(&site);
    assert_eq!(ed.editor().review().unwrap().len(), 0);
}

#[test]
fn a_default_level_edit_reviews_the_deterministic_fields() {
    let site = editing_site();
    let ed = editing(&site);
    let key = field(&ed.data(), Part::Party, "Aria", "Level").key;
    assert!(ed.editor().commit(key, text("34")).is_none());

    let rows = ed.editor().review().unwrap();
    assert_eq!(rows.iter().filter(|row| row.kind == ReviewKind::Heading).count(), 1);
    assert_eq!(rows[0], heading("Aria"));
    assert_eq!(find(&rows, "Level").before, "31");
    assert_eq!(find(&rows, "Level").after, "34");
    assert_eq!(
        (find(&rows, "Experience").before.as_str(), find(&rows, "Experience").after.as_str()),
        ("100100", "125357")
    );
    assert_eq!(
        (find(&rows, "Accuracy gain").before.as_str(), find(&rows, "Accuracy gain").after.as_str()),
        ("143", "155")
    );
    assert_eq!(
        (
            find(&rows, "Magic evasion gain").before.as_str(),
            find(&rows, "Magic evasion gain").after.as_str()
        ),
        ("133", "142")
    );
}

#[test]
fn no_save_open_answers_without_panic() {
    let site = editing_site();
    let ed = editing(&site);
    ed.editor().back();
    assert_eq!(ed.editor().history(), History::default());
    assert_eq!(ed.editor().undo().expect("a notice").problem, EditProblem::NotEditable);
    assert_eq!(ed.editor().redo().expect("a notice").problem, EditProblem::NotEditable);
    assert!(ed.editor().review().is_err());
}

#[test]
fn edits_are_refused_while_a_save_runs() {
    let site = editing_site();
    let ed = editing(&site);
    let data = tempfile::tempdir().unwrap();
    ed.flow.state.set_data_dir(neoffse::io::dirs::DataDir::at(data.path()));
    ed.editor().set_process_probe(|_| Vec::new());
    set_gil(&ed, "500");
    let key = field(&ed.data(), Part::Inventory, "Gil", "Gil").key;

    let outcomes = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&outcomes);
    let dispatcher: Arc<dyn Dispatcher> = ed.flow.manual.dispatcher();
    ed.editor().save(neoffse::Running::Refuse, &dispatcher, move |outcome| {
        log.lock().unwrap_or_else(PoisonError::into_inner).push(outcome);
    });
    assert!(ed.editor().is_saving());

    let refused = |notice: Option<neoffse_app::EditNotice>| assert_eq!(notice.expect("a notice").problem, EditProblem::NotEditable);
    refused(ed.editor().commit(key, text("600")));
    refused(ed.editor().add_row(0, 4));
    refused(ed.editor().remove_row(0));
    refused(ed.editor().undo());
    refused(ed.editor().redo());
    assert!(ed.editor().review().is_err());
    assert_eq!(ed.pending(), 1);

    ed.flow.manual.run_all();
    assert!(!ed.editor().is_saving());
    assert_eq!(outcomes.lock().unwrap_or_else(PoisonError::into_inner).len(), 1);
}
