//! What stops a restore before it writes, and how failures end.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use common::{Ended, Restoring, rewrite_manifest, slot_bytes, slot_file};
use neoffse::Running;
use neoffse::core::Game;
use neoffse_app::state::AppState;
use neoffse_app::{RestoreOutcome, RestoreViewModel};

/// A backup of slot 1 as `slot_bytes(1)`, then the folder holds `slot_bytes(2)`.
fn differing() -> Restoring {
    let r = Restoring::new();
    r.backup_now("before-save", 0);
    fs::write(r.file(), slot_bytes(2)).unwrap();
    r
}

fn probe_count(r: &Restoring) -> usize {
    r.probes.load(Ordering::SeqCst)
}

#[test]
fn a_missing_folder_changes_nothing() {
    let r = differing();
    r.pick(0);
    fs::remove_dir_all(&r.folder).unwrap();
    assert_eq!(r.restore(Running::Refuse), RestoreOutcome::FolderMissing);
    assert_eq!(probe_count(&r), 0, "no process scan");
    assert!(!r.folder.exists(), "no folder re-created");
    assert_eq!(r.reasons(), ["before-save"], "no safety backup");
}

#[test]
fn a_relative_source_dir_is_a_missing_folder() {
    let r = differing();
    let backup = r.backups(Game::Ff1Pr).remove(0);
    rewrite_manifest(&backup, "sourceDir", "saves/ff1");
    r.pick(0);
    let ended = Ended::default();
    r.vm.restore(Running::Refuse, &r.flow.manual.dispatcher(), ended.recorder());
    assert_eq!(ended.outcomes(), vec![RestoreOutcome::FolderMissing], "answered at once");
    assert_eq!(r.flow.manual.queued_jobs(), 0);
    assert!(!r.vm.is_busy());
    assert_eq!(r.bytes(), slot_bytes(2));
}

#[test]
fn a_running_game_is_refused() {
    let r = differing();
    r.pick(0);
    r.game_runs(true);
    assert_eq!(r.restore(Running::Refuse), RestoreOutcome::GameRunning);
    assert_eq!(r.bytes(), slot_bytes(2));
    assert_eq!(r.reasons(), ["before-save"]);
    assert!(r.vm.selected().is_some());

    assert!(matches!(r.restore(Running::Allow), RestoreOutcome::Restored { .. }));
    assert_eq!(r.bytes(), slot_bytes(1));
    assert_eq!(probe_count(&r), 1, "the first call probed, the second did not");
}

#[test]
fn allow_never_probes() {
    let r = differing();
    r.pick(0);
    r.game_runs(true);
    assert!(matches!(r.restore(Running::Allow), RestoreOutcome::Restored { .. }));
    assert_eq!(probe_count(&r), 0);
}

#[test]
fn an_error_before_the_first_write_is_a_failure() {
    let r = differing();
    let backup = r.backups(Game::Ff1Pr).remove(0);
    fs::remove_file(backup.dir.join(slot_file(1))).unwrap();
    r.pick(0);
    assert_eq!(r.restore(Running::Refuse), RestoreOutcome::Failed);
    assert_eq!(r.bytes(), slot_bytes(2));
    assert_eq!(r.reasons(), ["before-save"], "no safety backup");
    assert!(!r.vm.is_busy());
}

#[test]
fn no_selection_fails_at_once() {
    let r = differing();
    assert!(r.vm.show(Game::Ff1Pr).is_none());
    let ended = Ended::default();
    r.vm.restore(Running::Refuse, &r.flow.manual.dispatcher(), ended.recorder());
    assert_eq!(ended.outcomes(), vec![RestoreOutcome::Failed]);
    assert_eq!(r.flow.manual.queued_jobs(), 0);
    assert!(!r.vm.is_busy());
}

// `AppState` cannot unset its data folder, so a selection always comes from a
// state that had one. The "no data folder" step of `restore` is reachable only
// through the missing selection that a failed listing leaves: this test covers
// that path.
#[test]
fn no_data_folder_fails_at_once() {
    let r = Restoring::new();
    let vm = RestoreViewModel::new(common::registry(), Arc::new(AppState::new()));
    assert!(vm.show(Game::Ff1Pr).is_some());
    let ended = Ended::default();
    vm.restore(Running::Refuse, &r.flow.manual.dispatcher(), ended.recorder());
    assert_eq!(ended.outcomes(), vec![RestoreOutcome::Failed]);
    assert_eq!(r.flow.manual.queued_jobs(), 0);
    assert!(!vm.is_busy());
}

#[test]
fn a_second_restore_is_busy() {
    let r = differing();
    r.pick(0);
    let (first, second) = (Ended::default(), Ended::default());
    let dispatcher = r.flow.manual.dispatcher();
    r.vm.restore(Running::Refuse, &dispatcher, first.recorder());
    r.vm.restore(Running::Refuse, &dispatcher, second.recorder());
    assert_eq!(second.outcomes(), vec![RestoreOutcome::Busy]);
    assert_eq!(r.flow.manual.queued_jobs(), 1);
    assert!(r.vm.is_busy(), "the refused call left the flag alone");
    r.flow.manual.run_all();
    assert!(matches!(first.outcomes()[0], RestoreOutcome::Restored { .. }));
    assert!(!r.vm.is_busy());
}

#[test]
fn a_panicking_probe_is_incomplete() {
    let r = differing();
    r.pick(0);
    r.vm.set_process_probe(|_| panic!("scripted probe failure"));
    assert_eq!(r.restore(Running::Refuse), RestoreOutcome::Incomplete);
    assert!(!r.vm.is_busy());
    assert_eq!(r.bytes(), slot_bytes(2));
    assert!(r.vm.selected().is_some());
    r.vm.set_process_probe(|_| Vec::new());
    assert!(matches!(r.restore(Running::Refuse), RestoreOutcome::Restored { .. }));
}

#[cfg(unix)]
#[test]
fn a_write_failure_after_the_safety_backup_is_incomplete() {
    use std::os::unix::fs::PermissionsExt;

    let r = differing();
    r.pick(0);
    fs::set_permissions(&r.folder, fs::Permissions::from_mode(0o555)).unwrap();
    let probe = r.folder.join("probe");
    if fs::write(&probe, b"x").is_ok() {
        fs::remove_file(&probe).unwrap();
        fs::set_permissions(&r.folder, fs::Permissions::from_mode(0o755)).unwrap();
        neoffse_testkit::skip("the process ignores folder permissions (root)");
        return;
    }
    let outcome = r.restore(Running::Refuse);
    fs::set_permissions(&r.folder, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(outcome, RestoreOutcome::Incomplete);
    assert_eq!(r.reasons(), ["before-restore", "before-save"]);
    assert_eq!(r.bytes(), slot_bytes(2));
}
