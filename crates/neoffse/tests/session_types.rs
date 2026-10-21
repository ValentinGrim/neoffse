//! `Session`: thread bound and `Debug`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{open_session, slot_bytes, slot_name};
use neoffse::Session;
use neoffse::core::{Edit, FieldId, Value};
use neoffse::pr::ff1::names;

fn is_send<T: Send>() {}

#[test]
fn a_session_crosses_threads() {
    is_send::<Session>();
}

#[test]
fn debug_holds_the_game_and_the_name_only() {
    let (dir, mut session) = open_session();
    let edit = Edit::set(FieldId::new(names::GIL), Value::Int(1));
    session.edit(&edit).unwrap();
    let text = format!("{session:?}");
    assert!(text.contains("ff1-pr"));
    assert!(text.contains(slot_name(2)));
    assert!(text.contains("dirty: true"));
    // No byte of the file, and no folder path.
    let bytes = slot_bytes();
    let body = String::from_utf8_lossy(&bytes[3..40]).into_owned();
    assert!(!text.contains(&body));
    assert!(!text.contains(&dir.path().display().to_string()));
}
