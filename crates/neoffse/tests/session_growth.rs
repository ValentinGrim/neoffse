//! A level increase under `Session::edit` grows the stats with a random draw,
//! and undo and redo replay the drawn values
//! (`docs/formats/pixel-remaster.md#level-up-in-ff-i`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::open_session;
use neoffse::core::{Edit, FieldId, Value};
use neoffse::pr::ff1::names;

fn field(name: &'static str) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(1).with_name(name)
}

fn set(field: &FieldId, value: i64) -> Edit {
    Edit::set(field.clone(), Value::Int(value))
}

fn int(view: &neoffse::core::SaveView, name: &'static str) -> i64 {
    let (_, found) = view.find(&field(name)).expect("the field is in the view");
    let Value::Int(number) = found.value else { panic!("{name} is an integer") };
    number
}

#[test]
fn undo_and_redo_replay_the_drawn_values() {
    let (_dir, mut session) = open_session();
    let view_before = session.view().unwrap();
    // The synthetic slot holds level 31: three steps run.
    session.edit(&set(&field(names::LEVEL), 34)).unwrap();
    let view_after = session.view().unwrap();
    assert_eq!(int(&view_after, names::LEVEL), 34);
    assert_eq!(int(&view_after, names::ACCURACY_GAIN), 155);
    assert_ne!(view_after, view_before);
    assert_eq!(session.pending_count(), 1);

    assert!(session.undo().unwrap());
    assert_eq!(session.view().unwrap(), view_before);
    assert!(session.redo().unwrap());
    assert_eq!(session.view().unwrap(), view_after, "the drawn values come back, nothing is redrawn");
}

#[test]
fn a_level_edit_in_expert_mode_grows_nothing() {
    let (_dir, mut session) = open_session();
    let before = session.view().unwrap();
    session.edit_expert(&set(&field(names::LEVEL), 34)).unwrap();
    let after = session.view().unwrap();
    assert_eq!(int(&after, names::LEVEL), 34);
    for name in [names::MAX_HP_GAIN, names::POWER, names::ACCURACY_GAIN, names::MAGIC_EVASION_GAIN, names::EXP] {
        assert_eq!(int(&after, name), int(&before, name), "{name}");
    }
}
