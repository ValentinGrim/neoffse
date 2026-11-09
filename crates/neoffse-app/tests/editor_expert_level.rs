//! The spell Level in expert mode: the field has no stored place of its own,
//! so a commit reaches the game interface in either mode, writes the same
//! places and never sets the raw flag of the review. One-sided places keep
//! their expert route. No window shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Editing, Part, ff2_editing, ff2_site, field};
use neoffse::core::Capability;
use neoffse::data::Language;
use neoffse_app::editor::Input;
use neoffse_app::{ReviewKind, ReviewRow};

fn field_rows(rows: &[ReviewRow]) -> usize {
    rows.iter().filter(|row| row.kind == ReviewKind::Field).count()
}

fn commit_level(ed: &Editing, text: &str) {
    let key = field(&ed.data(), Part::Magic, "Ivo", "Fire").key;
    assert!(ed.editor().commit(key, Input::Text(text.to_owned())).is_none());
}

#[test]
fn an_expert_level_commit_writes_the_places_of_the_default_commit() {
    let default_site = ff2_site();
    let default = ff2_editing(&default_site);
    commit_level(&default, "9");
    let expected = default.with_session(|session| session.changes(Language::En).unwrap());
    assert_eq!(expected.len(), 9);

    let expert_site = ff2_site();
    let expert = ff2_editing(&expert_site);
    expert.editor().set_expert(true);
    commit_level(&expert, "9");
    let found = expert.with_session(|session| session.changes(Language::En).unwrap());
    assert!(found == expected, "the expert commit wrote other places");
    assert!(found.iter().all(|change| change.capability == Capability::Magic));
    assert_eq!(field(&expert.data(), Part::Magic, "Ivo", "Fire").text, "9");
    assert_eq!(expert.pending(), 1);
}

#[test]
fn undo_of_an_expert_level_commit_restores_every_byte() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    ed.editor().set_expert(true);
    let before = ed.with_session(|session| session.view().unwrap());
    commit_level(&ed, "9");
    assert!(ed.editor().undo().is_none());
    let after = ed.with_session(|session| session.view().unwrap());
    assert!(after == before, "the view differs after the undo");
    assert_eq!(ed.with_session(|session| session.changes(Language::En).unwrap()).len(), 0);
    assert!(ed.editor().redo().is_none());
    assert_eq!(field(&ed.data(), Part::Magic, "Ivo", "Fire").text, "9");
}

#[test]
fn the_review_of_an_expert_level_commit_follows_the_mode() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    ed.editor().set_expert(true);
    commit_level(&ed, "9");
    assert_eq!(field_rows(&ed.editor().review().unwrap()), 9);
    ed.editor().set_expert(false);
    let rows = ed.editor().review().unwrap();
    assert_eq!(field_rows(&rows), 1);
    assert_eq!((rows[1].label.as_str(), rows[1].before.as_str(), rows[1].after.as_str()), ("Fire", "5", "9"));
}

#[test]
fn a_typed_99_clamps_to_the_top_level_in_expert_mode() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    ed.editor().set_expert(true);
    commit_level(&ed, "99");
    assert_eq!(field(&ed.data(), Part::Magic, "Ivo", "Fire").text, "16");
}

#[test]
fn a_one_sided_commit_keeps_the_expert_route_and_its_review_row() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    ed.editor().set_expert(true);
    commit_level(&ed, "9");
    let place = field(&ed.data(), Part::RawMagic, "Ivo", "List 2, Fire, value").key;
    assert!(ed.editor().commit(place, Input::Text("456".to_owned())).is_none());
    ed.editor().set_expert(false);
    let rows = ed.editor().review().unwrap();
    assert!(field_rows(&rows) > 1, "the raw rows stay after expert mode ends");
    assert!(ed.editor().undo().is_none());
    assert!(ed.editor().undo().is_none());
    assert_eq!(ed.pending(), 0);
}
