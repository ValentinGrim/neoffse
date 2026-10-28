//! `Session::save` on temporary copies of the owner's FF I saves
//! (`NEOFFSE_SAVES`, read only). The test skips when the folder holds no
//! slot file and prints slot positions, never file names or values.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{assert_gil_only_change, data_dir, gil, gil_value, moment, must, no_process, private_slot_copies, set};
use neoffse::core::Game;
use neoffse::data::Language;
use neoffse::io::atomic::RealFileOps;
use neoffse::{Registry, Running};

#[test]
fn a_gil_edit_saves_on_every_private_slot_copy() {
    let Some(copies) = private_slot_copies() else { return };
    let registry = Registry::new(Language::En);
    let (_data_root, data) = data_dir();

    for (index, (name, copy)) in copies.iter().enumerate() {
        let name = name.as_str();
        let mut session = must(registry.open(copy.path(), name, Some(Game::Ff1Pr)), "open a private slot copy");
        let before = gil_value(&mut session);
        let next = if before > 0 { before - 1 } else { 1 };
        must(session.edit(&set(&gil(), next)), "edit the gil");
        let original = session.original().clone();

        let saved = must(
            session.save_with(&RealFileOps, no_process, moment(), &data, Running::Refuse),
            "save a private slot copy",
        );
        assert!(saved.backup.is_some(), "slot {index} must take a backup");
        assert!(!session.is_dirty(), "slot {index} must be clean after the save");

        let mut reopened = must(registry.open(copy.path(), name, Some(Game::Ff1Pr)), "reopen the saved copy");
        assert!(gil_value(&mut reopened) == next, "slot {index} must keep the new gil");
        assert_gil_only_change(&original, next, copy.path(), name);
    }
}
