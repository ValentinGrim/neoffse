//! `Session::changes` on the standard slot: the gil and the level.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{data_dir, gil, gil_value, level, moment, no_process, open_session, set};
use neoffse::core::{Capability, FieldId};
use neoffse::data::Language;
use neoffse::io::atomic::RealFileOps;
use neoffse::pr::ff1::names;
use neoffse::{Change, ChangeKind, Running};

fn gil_change(before: &str, after: &str) -> Change {
    let kind = ChangeKind::Field {
        field: gil(),
        label: "Gil".to_owned(),
        before: before.to_owned(),
        after: after.to_owned(),
    };
    Change::new(Capability::Inventory, FieldId::new(names::GIL), "Gil", kind)
}

fn field_texts(changes: &[Change], label: &str) -> Option<(String, String)> {
    changes.iter().find_map(|change| match &change.kind {
        ChangeKind::Field {
            label: found, before, after, ..
        } if found == label => Some((before.clone(), after.clone())),
        _ => None,
    })
}

#[test]
fn a_clean_session_has_no_change() {
    let (_dir, mut session) = open_session();
    assert_eq!(session.changes(Language::En).unwrap(), Vec::<neoffse::Change>::new());
}

#[test]
fn a_gil_edit_shows_one_change() {
    let (_dir, mut session) = open_session();
    assert_eq!(gil_value(&mut session), 123_456);
    session.edit(&set(&gil(), 123_463)).unwrap();
    assert_eq!(session.changes(Language::En).unwrap(), [gil_change("123456", "123463")]);
}

#[test]
fn a_value_set_back_is_no_change() {
    let (_dir, mut session) = open_session();
    session.edit(&set(&gil(), 123_457)).unwrap();
    session.edit(&set(&gil(), 123_456)).unwrap();
    assert_eq!(session.pending_count(), 2);
    assert_eq!(session.changes(Language::En).unwrap(), Vec::<neoffse::Change>::new());
}

#[test]
fn an_undo_to_the_clean_point_clears_the_changes_and_a_redo_brings_them_back() {
    let (_dir, mut session) = open_session();
    session.edit(&set(&gil(), 123_463)).unwrap();
    assert!(session.undo().unwrap());
    assert_eq!(session.changes(Language::En).unwrap(), Vec::<neoffse::Change>::new());
    assert!(session.redo().unwrap());
    assert_eq!(session.changes(Language::En).unwrap(), [gil_change("123456", "123463")]);
}

#[test]
fn an_expert_level_edit_shows_the_level_only() {
    let (_dir, mut session) = open_session();
    session.edit_expert(&set(&level(), 34)).unwrap();
    let kind = ChangeKind::Field {
        field: level(),
        label: "Level".to_owned(),
        before: "31".to_owned(),
        after: "34".to_owned(),
    };
    let aria = Change::new(Capability::Party, FieldId::new(names::CHARACTER).with_id(1), "Aria", kind);
    assert_eq!(session.changes(Language::En).unwrap(), [aria]);
}

#[test]
fn a_default_level_edit_shows_the_link_and_the_growth() {
    let (_dir, mut session) = open_session();
    session.edit(&set(&level(), 34)).unwrap();
    let changes = session.changes(Language::En).unwrap();
    assert!(changes.iter().all(|change| change.capability == Capability::Party));

    let pair = |a: &str, b: &str| Some((a.to_owned(), b.to_owned()));
    assert_eq!(field_texts(&changes, "Level"), pair("31", "34"));
    assert_eq!(field_texts(&changes, "Experience"), pair("100100", "125357"));
    assert_eq!(field_texts(&changes, "Accuracy gain"), pair("143", "155"));
    assert_eq!(field_texts(&changes, "Magic evasion gain"), pair("133", "142"));

    let (before, after) = field_texts(&changes, "Max HP gain").expect("the HP gain is listed");
    assert!(after.parse::<i64>().unwrap() > before.parse::<i64>().unwrap());

    let allowed = [
        "Level",
        "Experience",
        "Accuracy gain",
        "Magic evasion gain",
        "Max HP gain",
        "Power",
        "Vitality",
        "Agility",
        "Intelligence",
        "Spirit",
        "Luck",
        "Magic",
    ];
    for change in &changes {
        let ChangeKind::Field { label, .. } = &change.kind else {
            panic!("only field changes")
        };
        let is_charge = (1..=8).any(|level| label == &format!("Max charges gain, level {level}"));
        assert!(is_charge || allowed.contains(&label.as_str()), "unexpected line {label:?}");
        assert!(label != "Max HP" && label != "Next level", "a read-only line {label:?}");
    }
}

#[test]
fn changes_leave_the_session_alone() {
    let (_dir, mut session) = open_session();
    session.edit(&set(&gil(), 123_463)).unwrap();
    session.undo().unwrap();
    session.edit(&set(&gil(), 123_470)).unwrap();
    let view = session.view().unwrap();
    let state = (session.pending_count(), session.can_undo(), session.can_redo());

    let first = session.changes(Language::En).unwrap();
    let second = session.changes(Language::En).unwrap();
    assert_eq!(first, second);
    assert_eq!((session.pending_count(), session.can_undo(), session.can_redo()), state);
    assert_eq!(session.view().unwrap(), view);
}

#[test]
fn a_save_moves_the_clean_point() {
    let (_dir, mut session) = open_session();
    let (_data_dir, data) = data_dir();
    session.edit(&set(&gil(), 123_463)).unwrap();
    session.save_with(&RealFileOps, no_process, moment(), &data, Running::Allow).unwrap();
    assert_eq!(session.changes(Language::En).unwrap(), Vec::<neoffse::Change>::new());

    assert!(session.undo().unwrap());
    assert_eq!(session.changes(Language::En).unwrap(), [gil_change("123463", "123456")]);
    assert!(session.redo().unwrap());
    assert_eq!(session.changes(Language::En).unwrap(), Vec::<neoffse::Change>::new());
}

#[test]
fn a_discard_clears_the_changes() {
    let (_dir, mut session) = open_session();
    session.edit(&set(&gil(), 123_463)).unwrap();
    assert_eq!(session.changes(Language::En).unwrap().len(), 1);
    session.discard().unwrap();
    assert_eq!(session.changes(Language::En).unwrap(), Vec::<neoffse::Change>::new());
}
