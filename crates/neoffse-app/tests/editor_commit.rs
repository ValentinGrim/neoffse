//! `EditorViewModel::commit`: one edit per commit, the mode, the errors.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Editing, Part, count_of, editing, editing_site, field, item_label, list, pages, row_of, rows, spare_weapon};
use neoffse::core::{Edit, FieldId, Value};
use neoffse::data::Language;
use neoffse_app::EditProblem;
use neoffse_app::editor::Input;

fn text(value: &str) -> Input {
    Input::Text(value.to_owned())
}

fn gil_key(ed: &Editing) -> i32 {
    field(&ed.data(), Part::Inventory, "Gil", "Gil").key
}

fn party_key(ed: &Editing, page: &str, label: &str) -> i32 {
    field(&ed.data(), Part::Party, page, label).key
}

#[test]
fn a_commit_makes_one_history_entry_and_refreshes() {
    let site = editing_site();
    let ed = editing(&site);
    let key = gil_key(&ed);

    assert!(ed.editor().commit(key, text("500")).is_none());
    assert_eq!(ed.pending(), 1);
    assert!(ed.editor().is_dirty());
    assert_eq!(ed.gil(), "500");

    ed.undo();
    assert_eq!(ed.gil(), "123456");
    assert!(!ed.editor().is_dirty());
}

#[test]
fn an_unchanged_value_sends_nothing() {
    let site = editing_site();
    let ed = editing(&site);
    assert!(ed.editor().commit(gil_key(&ed), text("123456")).is_none());
    assert_eq!(ed.pending(), 0);
    assert!(
        ed.editor().commit(gil_key(&ed), text(" +123456 ")).is_none(),
        "the parsed value equals the shown one"
    );
    assert_eq!(ed.pending(), 0);
}

#[test]
fn text_commits_rename() {
    let site = editing_site();
    let ed = editing(&site);
    let key = party_key(&ed, "Aria", "Name");
    assert!(ed.editor().commit(key, text("Zed")).is_none());

    let data = ed.data();
    let labels: Vec<String> = pages(&data.party).iter().map(|page| page.label.to_string()).collect();
    assert_eq!(labels, ["Zed", "Bram", "Cleo", "Zoé"]);
    assert_eq!(ed.pending(), 1);
}

#[test]
fn typing_clamps_to_the_storage_bounds() {
    let site = editing_site();
    let ed = editing(&site);
    let key = gil_key(&ed);
    let cap = field(&ed.data(), Part::Inventory, "Gil", "Gil").cap.to_string();
    assert_ne!(cap, "");

    assert!(ed.editor().commit(key, text("-5")).is_none());
    assert_eq!(ed.gil(), "0");
    assert_eq!(ed.pending(), 1);

    let notice = ed.editor().commit(key, text("99999999999")).expect("above the cap");
    assert_eq!(notice.problem, EditProblem::AboveCap);
    assert_eq!(notice.value, "2147483647");
    assert_eq!(notice.cap.to_string(), cap);
    assert_eq!(notice.label, "Gil");
    assert_eq!(ed.pending(), 1, "the refused edit left no entry");
    assert_eq!(ed.gil(), "0");
}

#[test]
fn bad_text_is_refused_before_the_facade() {
    let site = editing_site();
    let ed = editing(&site);
    let notice = ed.editor().commit(gil_key(&ed), text("abc")).expect("not a number");
    assert_eq!(notice.problem, EditProblem::NotANumber);
    assert_eq!(notice.label, "Gil");
    assert_eq!(ed.pending(), 0);
}

#[test]
fn above_cap_is_refused_in_the_default_mode_and_accepted_in_expert() {
    let site = editing_site();
    let ed = editing(&site);
    let key = gil_key(&ed);

    let notice = ed.editor().commit(key, text("1000000")).expect("above the cap");
    assert_eq!(notice.problem, EditProblem::AboveCap);
    assert_eq!(notice.cap, "999999");
    assert_eq!(notice.value, "1000000");
    assert_eq!(ed.pending(), 0);

    ed.editor().set_expert(true);
    assert!(ed.editor().commit(key, text("1000000")).is_none());
    assert_eq!(ed.pending(), 1);
    assert!(field(&ed.data(), Part::Inventory, "Gil", "Gil").above_cap);
}

#[test]
fn a_level_above_the_cap_is_clamped_to_it_in_both_modes() {
    let site = editing_site();
    let ed = editing(&site);
    let key = party_key(&ed, "Aria", "Level");

    assert!(ed.editor().commit(key, text("100")).is_none());
    assert_eq!(field(&ed.data(), Part::Party, "Aria", "Level").text, "99");
    assert_eq!(ed.pending(), 1);

    ed.editor().set_expert(true);
    let key = party_key(&ed, "Aria", "Level");
    assert!(ed.editor().commit(key, text("500")).is_none());
    let level = field(&ed.data(), Part::Party, "Aria", "Level");
    assert_eq!(level.text, "99");
    assert!(!level.above_cap);
}

#[test]
fn a_floor_error_prints_the_errors_own_range() {
    let site = editing_site();
    let ed = editing(&site);
    let key = count_of(&row_of(&ed.data(), "Items", 183)).key;

    let notice = ed.editor().commit(key, text("1")).expect("below the wearers");
    assert_eq!(notice.problem, EditProblem::OutOfRange);
    assert_eq!(notice.min, "2");
    assert_eq!(notice.max, "2147483647");
    assert_eq!(notice.value, "1");
    assert_eq!(notice.label, item_label(183, "Item"), "the notice names the item");
    assert_eq!(ed.pending(), 0);
}

#[test]
fn a_locked_field_reaches_the_facade_and_is_refused() {
    let site = editing_site();
    let ed = editing(&site);
    let slot = field(&ed.data(), Part::Party, "Aria", "Count in slot 1");
    let next = (slot.text.parse::<i64>().unwrap() + 1).to_string();

    let notice = ed.editor().commit(slot.key, text(&next)).expect("locked");
    assert_eq!(notice.problem, EditProblem::ReadOnly);
    assert_eq!(ed.pending(), 0);

    ed.editor().set_expert(true);
    assert!(ed.editor().commit(slot.key, text(&next)).is_none());
    assert_eq!(ed.pending(), 1);
}

#[test]
fn magic_is_read_only_in_both_modes() {
    let site = editing_site();
    let ed = editing(&site);
    let key = field(&ed.data(), Part::Magic, "Aria", "Level 1, slot 1").key;
    for expert in [false, true] {
        ed.editor().set_expert(expert);
        let notice = ed.editor().commit(key, text("5")).expect("read only");
        assert_eq!(notice.problem, EditProblem::ReadOnly);
        assert_eq!(ed.pending(), 0);
    }
}

#[test]
fn a_picker_commit_equips_in_one_entry() {
    let site = editing_site();
    let ed = editing(&site);
    let id = spare_weapon(&ed.data());
    let before = ed.data();
    let key = field(&before, Part::Party, "Aria", "Right hand").key;
    let rows_before = rows(&list(&before, "Items")).len();

    assert!(ed.editor().commit(key, Input::Pick(i64::from(id))).is_none());
    assert_eq!(ed.pending(), 1, "the three steps of the plan are one entry");

    let after = ed.data();
    assert_eq!(rows(&list(&after, "Items")).len(), rows_before + 1);
    let added = count_of(&row_of(&after, "Items", id));
    assert_eq!(added.text, "1");
    let slot = field(&after, Part::Party, "Aria", "Right hand");
    assert_eq!(slot.text.to_string(), id.to_string());
    assert_eq!(field(&after, Part::Party, "Aria", "Count in slot 1").text, "1");

    ed.undo();
    let restored = ed.data();
    assert_eq!(rows(&list(&restored, "Items")).len(), rows_before);
    assert_eq!(
        field(&restored, Part::Party, "Aria", "Right hand").text,
        field(&before, Part::Party, "Aria", "Right hand").text
    );
    assert_eq!(
        field(&restored, Part::Party, "Aria", "Count in slot 1").text,
        field(&before, Part::Party, "Aria", "Count in slot 1").text
    );
}

#[test]
fn the_expert_equip_is_one_raw_set() {
    let site = editing_site();
    let ed = editing(&site);
    let id = spare_weapon(&ed.data());
    let before = ed.data();
    let key = field(&before, Part::Party, "Aria", "Right hand").key;
    let rows_before = rows(&list(&before, "Items")).len();

    ed.editor().set_expert(true);
    assert!(ed.editor().commit(key, Input::Pick(i64::from(id))).is_none());
    assert_eq!(ed.pending(), 1);

    let after = ed.data();
    assert_eq!(field(&after, Part::Party, "Aria", "Right hand").text.to_string(), id.to_string());
    assert_eq!(rows(&list(&after, "Items")).len(), rows_before, "no inventory row is added");
    assert_eq!(
        field(&after, Part::Party, "Aria", "Count in slot 1").text,
        field(&before, Part::Party, "Aria", "Count in slot 1").text
    );
}

#[test]
fn an_unknown_table_id_is_refused() {
    let site = editing_site();
    let ed = editing(&site);
    let key = party_key(&ed, "Aria", "Job");
    let notice = ed.editor().commit(key, Input::Pick(13)).expect("no such job");
    assert_eq!(notice.problem, EditProblem::UnknownId);
    assert_eq!(notice.value, "13");
    assert_eq!(ed.pending(), 0);
}

#[test]
fn a_stale_key_is_not_editable() {
    let site = editing_site();
    let ed = editing(&site);
    for key in [9999, -1] {
        let notice = ed.editor().commit(key, text("1")).expect("no such key");
        assert_eq!(notice.problem, EditProblem::NotEditable);
    }
    let key = gil_key(&ed);
    ed.editor().back();
    let notice = ed.editor().commit(key, text("1")).expect("the save is closed");
    assert_eq!(notice.problem, EditProblem::NotEditable);
}

#[test]
fn commit_and_data_see_the_same_session() {
    let site = editing_site();
    let ed = editing(&site);
    ed.with_session(|session| session.edit(&Edit::set(FieldId::new("gil"), Value::Int(777))))
        .unwrap();
    assert_eq!(ed.gil(), "777");
    assert!(ed.editor().is_dirty());
}

#[test]
fn the_language_does_not_change_the_commit() {
    // The same edit through a French model reaches the same document.
    let site = editing_site();
    let ed = editing(&site);
    let french = neoffse_app::EditorViewModel::new(std::sync::Arc::clone(&ed.flow.state), Language::Fr);
    let data = french.data().unwrap();
    let key = field(&data, Part::Inventory, "Gil", "Gil").key;
    assert!(french.commit(key, text("42")).is_none());
    assert_eq!(ed.gil(), "42");
}
