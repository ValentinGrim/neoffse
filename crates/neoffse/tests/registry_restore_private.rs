//! `Registry::restore` on temporary copies of the owner's FF I saves
//! (`NEOFFSE_SAVES`, read only). The test skips when the folder holds no
//! slot file and prints slot positions, never file names or values.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{data_dir, gil, gil_value, moment, must, no_process, private_slot_copies, set};
use neoffse::core::Game;
use neoffse::data::Language;
use neoffse::io::atomic::RealFileOps;
use neoffse::{Registry, Running};

#[test]
fn restoring_the_before_save_backup_returns_every_private_slot_copy() {
    let Some(copies) = private_slot_copies() else { return };
    let registry = Registry::new(Language::En);

    for (index, (name, copy)) in copies.iter().enumerate() {
        let name = name.as_str();
        let (_data_root, data) = data_dir();
        let original = must(fs::read(copy.path().join(name)), "read a private slot copy");
        let mut session = must(registry.open(copy.path(), name, Some(Game::Ff1Pr)), "open a private slot copy");
        let before = gil_value(&mut session);
        let next = if before > 0 { before - 1 } else { 1 };
        must(session.edit(&set(&gil(), next)), "edit the gil");
        let saved = must(
            session.save_with(&RealFileOps, no_process, moment(), &data, Running::Refuse),
            "save a private slot copy",
        );
        let backup = saved.backup.expect("a dirty save takes a backup");
        let edited = must(fs::read(copy.path().join(name)), "read the saved copy");
        assert!(edited != original, "slot {index} must change");

        must(
            registry.restore_with(&RealFileOps, no_process, moment(), &data, &backup, copy.path(), Running::Allow),
            "restore the backup",
        );
        let restored = must(fs::read(copy.path().join(name)), "read the restored copy");
        assert!(restored == original, "slot {index} must return byte for byte");
    }
}
