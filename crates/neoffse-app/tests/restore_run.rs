//! The restore on a worker: bytes, safety backups, retention, open-file flag.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::thread;

use common::{Ended, Restoring, T0, make_backup, slot_bytes, slot_file};
use neoffse::Running;
use neoffse::core::Game;
use neoffse_app::RestoreOutcome;
use neoffse_app::editor::stamp_of;

/// A backup of slot 1 as `slot_bytes(1)`, then the folder holds `slot_bytes(2)`.
fn differing() -> Restoring {
    let r = Restoring::new();
    r.backup_now("before-save", 0);
    fs::write(r.file(), slot_bytes(2)).unwrap();
    r
}

#[test]
fn a_restore_writes_the_backup_over_the_file() {
    let r = differing();
    r.pick(0);
    let ended = Ended::default();
    r.vm.restore(Running::Refuse, &r.flow.manual.dispatcher(), ended.recorder());
    assert!(r.vm.is_busy());
    assert_eq!(r.bytes(), slot_bytes(2), "nothing is written before the worker runs");
    let worker = r.flow.manual.run_jobs();
    r.flow.manual.run_posts();

    assert_eq!(r.bytes(), slot_bytes(1));
    assert!(!r.vm.is_busy());
    let all = ended.all();
    assert_eq!(all.len(), 1);
    assert_ne!(worker[0], thread::current().id());
    assert_eq!(all[0].1, thread::current().id());

    let newest = r.backups(Game::Ff1Pr).remove(0);
    assert_eq!(newest.manifest.reason, "before-restore");
    assert_eq!(fs::read(newest.dir.join(slot_file(1))).unwrap(), slot_bytes(2));
    let expected = stamp_of(&newest.manifest.created);
    assert!(expected.is_some());
    assert_eq!(
        all[0].0,
        RestoreOutcome::Restored {
            safety: expected,
            replaced_open: false
        }
    );
}

#[test]
fn undoing_a_restore_restores_the_safety_backup() {
    let r = differing();
    r.pick(0);
    assert!(matches!(r.restore(Running::Refuse), RestoreOutcome::Restored { .. }));
    assert_eq!(r.bytes(), slot_bytes(1));
    r.pick(0);
    assert!(!r.vm.selected().unwrap().stamp.is_empty());
    assert!(matches!(r.restore(Running::Refuse), RestoreOutcome::Restored { .. }));
    assert_eq!(r.bytes(), slot_bytes(2));
}

#[test]
fn a_restore_into_a_folder_without_the_file_makes_no_safety_backup() {
    let r = differing();
    fs::remove_file(r.file()).unwrap();
    r.pick(0);
    assert_eq!(
        r.restore(Running::Refuse),
        RestoreOutcome::Restored {
            safety: None,
            replaced_open: false
        }
    );
    assert_eq!(r.bytes(), slot_bytes(1));
    assert_eq!(r.reasons(), ["before-save"]);
}

#[test]
fn restoring_the_oldest_of_twenty_keeps_it() {
    let r = Restoring::new();
    for i in 0..20 {
        r.backup_now("before-save", i);
    }
    fs::write(r.file(), slot_bytes(2)).unwrap();
    r.pick(19);
    let oldest = r.vm.selected().unwrap().stamp.to_string();
    assert_eq!(oldest, "2020-09-13 12:26 UTC");
    assert!(matches!(r.restore(Running::Refuse), RestoreOutcome::Restored { .. }));

    let stamps: Vec<String> = r.backups(Game::Ff1Pr).iter().map(|b| stamp_of(&b.manifest.created).unwrap()).collect();
    assert_eq!(stamps.len(), 21);
    assert!(stamps.contains(&oldest), "the restored backup survives the pruning");
}

#[test]
fn a_broken_file_restores_byte_for_byte() {
    let r = Restoring::new();
    let garbage: Vec<u8> = (0..64u8).map(|i| i.wrapping_mul(7).wrapping_add(3)).collect();
    fs::write(r.file(), &garbage).unwrap();
    r.backup_now("before-save", 0);
    fs::write(r.file(), slot_bytes(2)).unwrap();
    r.pick(0);
    assert!(matches!(r.restore(Running::Refuse), RestoreOutcome::Restored { .. }));
    assert_eq!(r.bytes(), garbage);
}

#[test]
fn an_unsupported_game_restores() {
    let r = Restoring::new();
    let source = tempfile::tempdir().unwrap();
    let bytes = b"arbitrary bytes, no format".to_vec();
    fs::write(source.path().join("notes.bin"), &bytes).unwrap();
    make_backup(&r.data_dir(), "ff2-pr", source.path(), &["notes.bin"], "before-save", T0, 0);
    fs::write(source.path().join("notes.bin"), b"changed").unwrap();

    assert!(r.vm.show(Game::Ff2Pr).is_none());
    assert!(r.vm.select(0).is_some());
    assert!(matches!(r.restore(Running::Refuse), RestoreOutcome::Restored { .. }));
    assert_eq!(fs::read(source.path().join("notes.bin")).unwrap(), bytes);
}

#[test]
fn the_selection_clears_after_a_restore_and_stays_after_a_refusal() {
    let r = differing();
    r.pick(0);
    r.game_runs(true);
    assert_eq!(r.restore(Running::Refuse), RestoreOutcome::GameRunning);
    assert!(r.vm.selected().is_some());
    r.game_runs(false);
    assert!(matches!(r.restore(Running::Refuse), RestoreOutcome::Restored { .. }));
    assert!(r.vm.selected().is_none());
}

#[test]
fn replaced_open_is_true_for_the_open_file() {
    let r = Restoring::new();
    r.open_slot();
    r.backup_now("before-save", 0);
    r.pick(0);
    assert!(r.vm.replaces_open_save());
    assert!(matches!(r.restore(Running::Refuse), RestoreOutcome::Restored { replaced_open: true, .. }));
}

#[test]
fn another_slot_does_not_replace_the_open_file() {
    let r = Restoring::new();
    r.open_slot();
    fs::write(r.folder.join(slot_file(2)), slot_bytes(2)).unwrap();
    make_backup(&r.data_dir(), "ff1-pr", &r.folder, &[slot_file(2)], "before-save", T0, 0);
    r.pick(0);
    assert!(!r.vm.replaces_open_save());
    assert!(matches!(r.restore(Running::Refuse), RestoreOutcome::Restored { replaced_open: false, .. }));
}

#[test]
fn the_same_name_in_another_folder_does_not_replace_the_open_file() {
    let r = Restoring::new();
    r.open_slot();
    let elsewhere = tempfile::tempdir().unwrap();
    fs::write(elsewhere.path().join(slot_file(1)), slot_bytes(1)).unwrap();
    make_backup(&r.data_dir(), "ff1-pr", elsewhere.path(), &[slot_file(1)], "before-save", T0, 0);
    r.pick(0);
    assert!(!r.vm.replaces_open_save());
    assert!(matches!(r.restore(Running::Refuse), RestoreOutcome::Restored { replaced_open: false, .. }));
}

#[test]
fn no_open_session_never_replaces_it() {
    let r = differing();
    r.pick(0);
    assert!(!r.vm.replaces_open_save());
}
