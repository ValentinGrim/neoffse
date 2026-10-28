//! `Session::changes` on the owner's FF I saves (`NEOFFSE_SAVES`, read
//! only; the test edits in memory and never saves). The test skips when the
//! folder holds no slot file and prints slot positions and durations, never
//! file names or values.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::time::Instant;

use common::{gil, gil_value, must, must_some, private_slot_paths, set};
use neoffse::core::Game;
use neoffse::data::Language;
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
    let Some(slots) = private_slot_paths() else { return };
    let dir = must_some(slots.first().and_then(|(_, path)| path.parent()), "a slot file has a folder");
    let registry = Registry::new(Language::En);

    for (index, (name, _)) in slots.iter().enumerate() {
        let mut session = must(registry.open(dir, name, Some(Game::Ff1Pr)), "open a private slot");
        let before = gil_value(&mut session);
        let next = if before > 0 { before - 1 } else { 1 };
        must(session.edit(&set(&gil(), next)), "edit the gil");

        let started = Instant::now();
        let changes = must(session.changes(Language::En), "list the changes");
        report(&index.to_string(), started.elapsed().as_secs_f64() * 1000.0);
        assert!(changes.len() == 1, "slot {index} must show one change");
        assert!(matches!(changes[0].kind, ChangeKind::Field { .. }), "slot {index} must show a field change");

        assert!(must(session.undo(), "undo the gil edit"), "slot {index} must undo");
        let remaining = must(session.changes(Language::En), "list the changes after the undo");
        assert!(remaining.is_empty(), "slot {index} must show no change after the undo");
    }
}
