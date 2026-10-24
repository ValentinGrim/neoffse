//! The ways a restore ends without a plain success: a missing folder, an
//! error before the first write, a write that stops half way, a missing data
//! folder, a panicking probe, and the close request while it runs. Everything
//! runs on temporary folders under a scripted process probe.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{Paged, Ui, editing_site, listing, restore_page, slot_bytes, slot_file};
use neoffse::core::Game;
use neoffse_app::{BackupReason, RestoreProblem};
use slint::ComponentHandle;

#[test]
fn a_missing_folder_shows_the_folder_notice() {
    let (_site, p) = restore_page();
    fs::remove_dir_all(&p.folder).unwrap();
    p.start();
    p.ui.manual.run_all();

    assert_eq!(p.bridge().get_notice().problem, RestoreProblem::FolderMissing);
    assert!(p.bridge().get_has_notice());
    assert!(!p.folder.exists(), "no folder is created");
    assert_eq!(listing(&p.data_dir(), Game::Ff1Pr).len(), 1, "no safety backup");
    assert!(!p.bridge().get_working());
    assert_eq!(p.rows().len(), 1);
    assert_eq!(p.ui.manual.queued_jobs(), 0);
}

#[test]
fn a_failed_restore_shows_the_failure_notice() {
    let (_site, p) = restore_page();
    let backup = listing(&p.data_dir(), Game::Ff1Pr).remove(0);
    fs::remove_file(backup.dir.join(slot_file(1))).unwrap();
    p.start();
    p.ui.manual.run_all();

    assert_eq!(p.bridge().get_notice().problem, RestoreProblem::Failed);
    assert_eq!(p.file(), slot_bytes(2));
    assert!(!p.bridge().get_working());
    assert_eq!(listing(&p.data_dir(), Game::Ff1Pr).len(), 1);
}

#[cfg(unix)]
#[test]
fn an_incomplete_restore_shows_its_notice_refreshes_the_list_and_scans() {
    use std::os::unix::fs::PermissionsExt;

    let (_site, p) = restore_page();
    // A second slot that Home has not seen: only a new scan shows it.
    fs::write(p.folder.join(slot_file(2)), slot_bytes(2)).unwrap();
    assert_eq!(p.ui.first_slots().len(), 1);
    fs::set_permissions(&p.folder, fs::Permissions::from_mode(0o555)).unwrap();
    let probe = p.folder.join("probe");
    if fs::write(&probe, b"x").is_ok() {
        fs::remove_file(&probe).unwrap();
        fs::set_permissions(&p.folder, fs::Permissions::from_mode(0o755)).unwrap();
        neoffse_testkit::skip("the process ignores folder permissions (root)");
        return;
    }
    p.start();
    p.ui.manual.run_all();
    fs::set_permissions(&p.folder, fs::Permissions::from_mode(0o755)).unwrap();

    assert_eq!(p.bridge().get_notice().problem, RestoreProblem::Incomplete);
    assert!(!p.bridge().get_working());
    let rows = p.rows();
    assert_eq!(rows.len(), 2, "the list shows the safety backup");
    assert_eq!(rows[0].reason, BackupReason::BeforeRestore);
    assert_eq!(p.file(), slot_bytes(2));
    p.bridge().invoke_back_requested();
    assert_eq!(p.ui.first_slots().len(), 2, "Home scanned again after the partial restore");
}

#[test]
fn no_data_folder_is_a_failed_restore() {
    let site = editing_site();
    let ui = Ui::scanned(&site, None);
    ui.window.invoke_backups_requested("ff1-pr".into());
    assert!(ui.window.get_restore_open());
    // The page lists nothing without a data folder, so no row can start a
    // restore; the confirmation callback meets an empty selection.
    ui.restore_bridge().invoke_restore_confirmed();

    assert_eq!(ui.restore_bridge().get_notice().problem, RestoreProblem::Failed);
    assert!(ui.restore_bridge().get_has_notice());
    assert!(!ui.restore_bridge().get_working());
    assert_eq!(ui.manual.queued_jobs(), 0);
}

#[test]
fn a_panicking_probe_is_an_incomplete_restore() {
    let (_site, p) = restore_page();
    p.ui.app.restore.set_process_probe(|_| panic!("scripted probe failure"));
    p.start();
    p.ui.manual.run_all();

    assert_eq!(p.bridge().get_notice().problem, RestoreProblem::Incomplete);
    assert!(!p.bridge().get_working());
    assert!(!p.ui.app.restore.is_busy());
    assert_eq!(p.file(), slot_bytes(2));
}

#[test]
fn the_close_request_is_vetoed_while_a_restore_runs() {
    let site = editing_site();
    let (ui, _about) = Ui::with_about(&site);
    let p = Paged::over(ui, &site);
    p.backup("before-save", 0);
    fs::write(p.folder.join(slot_file(1)), slot_bytes(2)).unwrap();
    p.ui.window.show().unwrap();
    p.open_from_home();
    p.start();
    assert!(p.bridge().get_working());

    p.ui.window.window().dispatch_event(slint::platform::WindowEvent::CloseRequested);

    assert!(p.ui.window.window().is_visible());
    assert!(!p.ui.bridge().get_discard_dialog_open());
    p.ui.manual.run_all();
    assert_eq!(p.file(), slot_bytes(1));
    assert!(p.ui.window.window().is_visible());
}
