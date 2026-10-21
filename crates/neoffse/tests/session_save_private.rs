//! `Session::save` on temporary copies of the owner's FF I saves
//! (`NEOFFSE_SAVES`, read only). The test skips when the folder is absent
//! and prints slot labels, never file names.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{assert_gil_only_change, data_dir, gil, gil_value, moment, no_process, set};
use neoffse::core::Game;
use neoffse::data::Language;
use neoffse::io::atomic::RealFileOps;
use neoffse::pr::slots::SLOT_FILES;
use neoffse::{Registry, Running};

#[test]
fn a_gil_edit_saves_on_every_private_slot_copy() {
    let Some(paths) = neoffse_testkit::private_saves("ff1") else { return };
    let registry = Registry::new(Language::En);
    let (_data_root, data) = data_dir();

    for path in &paths {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !SLOT_FILES.iter().any(|file| file.name == name) {
            continue;
        }
        let copy = tempfile::tempdir().unwrap();
        fs::copy(path, copy.path().join(name)).unwrap();
        let mut session = registry.open(copy.path(), name, Some(Game::Ff1Pr)).unwrap();
        let slot = session.summary().unwrap().slot;
        let before = gil_value(&mut session);
        let next = if before > 0 { before - 1 } else { 1 };
        session.edit(&set(&gil(), next)).unwrap();
        let original = session.original().clone();

        let saved = session.save_with(&RealFileOps, no_process, moment(), &data, Running::Refuse).unwrap();
        assert!(saved.backup.is_some(), "slot {slot:?} must take a backup");
        assert!(!session.is_dirty(), "slot {slot:?} must be clean after the save");

        let mut reopened = registry.open(copy.path(), name, Some(Game::Ff1Pr)).unwrap();
        assert_eq!(gil_value(&mut reopened), next, "slot {slot:?} must keep the new gil");
        assert_gil_only_change(&original, next, copy.path(), name);
    }
}
