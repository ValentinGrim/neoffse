//! `Registry::new` with FF II: the two games open from their own slots, an
//! FF II session edits, undoes and redoes through the facade, and a file of a
//! game without a format stays unsupported.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::Path;

use common::{data_dir, ff2_slot_bytes, folder_with, moment, must, no_process, slot_bytes, slot_file_at, slot_with_vehicles};
use neoffse::core::{Capability, Edit, FieldId, Game, RowId, Value};
use neoffse::data::Language;
use neoffse::io::atomic::RealFileOps;
use neoffse::io::dirs::DataDir;
use neoffse::pr::ff2::names;
use neoffse::{Change, ChangeKind, Registry, Running, Session};
use tempfile::TempDir;

/// Character 1 of the fixture holds ability 711, then chain 0 at level 5.
const CHAIN: u32 = 0;
const TO: i64 = 1;
/// Position of the chain 0 entry in character 1's `abilityList`.
const ENTRY: u32 = 1;

fn registry() -> Registry {
    Registry::new(Language::En)
}

fn ff2_session() -> (TempDir, Session) {
    let dir = folder_with(&[(slot_file_at(2), &ff2_slot_bytes())]);
    let session = must(registry().open(dir.path(), slot_file_at(2), None), "open the FF II fixture");
    (dir, session)
}

/// Saves a dirty session and returns the bytes now on disk.
fn saved(session: &mut Session, dir: &Path, data: &DataDir) -> Vec<u8> {
    must(session.save_with(&RealFileOps, no_process, moment(), data, Running::Allow), "save the session");
    fs::read(dir.join(slot_file_at(2))).unwrap()
}

#[test]
fn the_ff2_fixture_opens_as_ff2_with_and_without_a_game() {
    let dir = folder_with(&[(slot_file_at(2), &ff2_slot_bytes())]);
    for game in [None, Some(Game::Ff2Pr)] {
        let session = must(registry().open(dir.path(), slot_file_at(2), game), "open the FF II fixture");
        assert_eq!(session.game(), Game::Ff2Pr);
    }
}

#[test]
fn the_ff1_fixture_still_opens_as_ff1() {
    let dir = folder_with(&[(slot_file_at(2), &slot_bytes())]);
    let session = must(registry().open(dir.path(), slot_file_at(2), None), "open the FF I fixture");
    assert_eq!(session.game(), Game::Ff1Pr);
}

#[test]
fn a_spell_level_edit_undoes_to_the_original_bytes_and_redoes() {
    let (dir, mut session) = ff2_session();
    let (_data_root, data) = data_dir();
    let original = fs::read(dir.path().join(slot_file_at(2))).unwrap();
    let level = names::spell_field(1, CHAIN);

    session.edit(&Edit::set(level, Value::Int(TO))).unwrap();
    let edited = saved(&mut session, dir.path(), &data);
    assert_ne!(edited, original);

    assert!(session.undo().unwrap());
    assert_eq!(saved(&mut session, dir.path(), &data), original);

    assert!(session.redo().unwrap());
    assert_eq!(saved(&mut session, dir.path(), &data), edited);
}

#[test]
fn a_one_sided_expert_edit_of_a_place_undoes_to_the_original_bytes() {
    let (dir, mut session) = ff2_session();
    let (_data_root, data) = data_dir();
    let original = fs::read(dir.path().join(slot_file_at(2))).unwrap();
    let place = names::place(1, names::ABILITY, ENTRY, names::PROFICIENCY);

    session.edit_expert(&Edit::set(place, Value::Int(456))).unwrap();
    assert_ne!(saved(&mut session, dir.path(), &data), original);

    assert!(session.undo().unwrap());
    assert_eq!(saved(&mut session, dir.path(), &data), original);
}

#[test]
fn a_keyword_add_undoes_to_the_original_bytes() {
    let (dir, mut session) = ff2_session();
    let (_data_root, data) = data_dir();
    let original = fs::read(dir.path().join(slot_file_at(2))).unwrap();
    // The fixture holds keyword rows 917, 918, 919 and 921: 920 is absent.
    let keywords = FieldId::new(names::KEYWORDS);

    session.edit(&Edit::insert(RowId::new(keywords, 920), None, Vec::new())).unwrap();
    assert_ne!(saved(&mut session, dir.path(), &data), original);

    assert!(session.undo().unwrap());
    assert_eq!(saved(&mut session, dir.path(), &data), original);
}

#[test]
fn the_changes_list_the_level_under_the_spell_name_on_the_magic_entity() {
    let (_dir, mut session) = ff2_session();
    session.edit(&Edit::set(names::spell_field(1, CHAIN), Value::Int(TO))).unwrap();

    let changes: Vec<Change> = session.changes(Language::En).unwrap();
    let level = names::spell_field(1, CHAIN);
    let (on_level, others): (Vec<&Change>, Vec<&Change>) = changes
        .iter()
        .partition(|change| matches!(&change.kind, ChangeKind::Field { field, .. } if *field == level));
    let [change] = on_level.as_slice() else {
        panic!("one Level change expected, got {}", on_level.len());
    };
    assert_eq!(change.capability, Capability::Magic);
    assert_eq!(change.entity, FieldId::new(names::CHARACTER).with_id(1));
    let ChangeKind::Field { label, before, after, .. } = &change.kind else {
        panic!("a field change expected");
    };
    assert_eq!((label.as_str(), before.as_str(), after.as_str()), ("Fire", "5", "1"));

    // The eight stored places of the edit sit on the character's raw page.
    let raw = FieldId::new(names::CHARACTER).with_id(1).with_name(names::RAW_SPELLS);
    assert_eq!(others.len(), 8);
    assert!(others.iter().all(|change| change.entity == raw));
}

#[test]
fn a_file_of_a_game_without_a_format_is_unsupported() {
    for vehicles in [14, 11, 19, 8] {
        let dir = folder_with(&[(slot_file_at(2), &slot_with_vehicles(vehicles))]);
        let Err(error) = registry().open(dir.path(), slot_file_at(2), None) else {
            panic!("{vehicles} vehicles must not open");
        };
        assert!(registry().is_unsupported_game(&error), "{vehicles} vehicles");
    }
}
