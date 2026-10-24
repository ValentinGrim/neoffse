//! `Session::changes` on the owner's FF I saves (`NEOFFSE_SAVES`, read
//! only; the test edits in memory and never saves). The test skips when the
//! folder is absent and prints slot labels and durations, never file names
//! or values.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::time::Instant;

use common::{gil, gil_value, set};
use neoffse::core::Game;
use neoffse::data::Language;
use neoffse::pr::slots::SLOT_FILES;
use neoffse::{ChangeKind, Registry};

#[allow(
    clippy::print_stderr,
    reason = "the timing of one call is the output of this test, and stderr is its only channel"
)]
fn report(slot: &str, milliseconds: f64) {
    eprintln!("changes on {slot}: {milliseconds:.2} ms");
}

#[test]
fn a_gil_edit_shows_one_change_on_every_private_slot() {
    let Some(paths) = neoffse_testkit::private_saves("ff1") else { return };
    let Some(dir) = paths.first().and_then(|path| path.parent()) else { return };
    let registry = Registry::new(Language::En);

    for path in &paths {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !SLOT_FILES.iter().any(|file| file.name == name) {
            continue;
        }
        let mut session = registry.open(dir, name, Some(Game::Ff1Pr)).unwrap();
        let slot = format!("{:?}", session.summary().unwrap().slot);
        let before = gil_value(&mut session);
        let next = if before > 0 { before - 1 } else { 1 };
        session.edit(&set(&gil(), next)).unwrap();

        let started = Instant::now();
        let changes = session.changes(Language::En).unwrap();
        report(&slot, started.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(changes.len(), 1, "slot {slot} must show one change");
        assert!(matches!(changes[0].kind, ChangeKind::Field { .. }), "slot {slot} must show a field change");

        assert!(session.undo().unwrap());
        assert_eq!(
            session.changes(Language::En).unwrap(),
            Vec::<neoffse::Change>::new(),
            "slot {slot} must show none after the undo"
        );
    }
}
