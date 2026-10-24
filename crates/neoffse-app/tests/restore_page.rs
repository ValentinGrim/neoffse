//! What the Restore page shows: the rows, the unreadable count, the notice,
//! the keys and the buttons. Everything runs on temporary folders. The text of
//! a notice is a `.slint` sentence the accessibility tree does not expose, so
//! the tests read the bridge data.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{Paged, Ui, editing_site};
use i_slint_backend_testing::AccessibleRole::Button;
use neoffse_app::{BackupReason, RestoreNotice, RestoreProblem, SlotKind};
use slint::Model;
use slint::platform::Key;

/// The page of FF I open from Home. The site must outlive the page.
fn paged_with_page() -> (common::Site, Paged) {
    let site = editing_site();
    let p = Paged::home(&site);
    p.open_from_home();
    (site, p)
}

#[test]
fn the_rows_show_stamp_slot_and_reason() {
    let site = editing_site();
    let p = Paged::home(&site);
    p.backup("before-save", 0);
    p.backup("before-restore", 1);
    p.backup("pre-upgrade", 2);
    p.open_from_home();

    let rows = p.rows();
    assert_eq!(rows[0].stamp.as_str(), "2020-09-13 12:28 UTC");
    let reasons: Vec<BackupReason> = rows.iter().map(|row| row.reason).collect();
    assert_eq!(reasons, [BackupReason::Other, BackupReason::BeforeRestore, BackupReason::BeforeSave]);
    assert!(rows.iter().all(|row| row.slot_kind == SlotKind::Numbered && row.slot_number == 1));
    let keys: Vec<i32> = rows.iter().map(|row| row.key).collect();
    assert_eq!(keys, [0, 1, 2]);
}

#[test]
fn unreadable_folders_show_as_a_count() {
    let site = editing_site();
    let p = Paged::home(&site);
    let backup = p.backup("before-save", 0);
    let parent = backup.dir.parent().unwrap();
    fs::create_dir(parent.join("20200101-000000")).unwrap();
    fs::create_dir(parent.join("20200102-000000")).unwrap();
    p.open_from_home();

    assert_eq!(p.bridge().get_unreadable_count(), 2);
    assert_eq!(p.rows().len(), 1);
}

#[test]
fn an_empty_list_has_no_notice() {
    let (_site, p) = paged_with_page();
    assert_eq!(p.rows().len(), 0);
    assert!(!p.bridge().get_has_notice());
    assert_eq!(p.bridge().get_unreadable_count(), 0);
}

#[test]
fn no_data_folder_shows_the_listing_notice() {
    let site = editing_site();
    let ui = Ui::scanned(&site, None);
    ui.window.invoke_backups_requested("ff1-pr".into());

    assert!(ui.window.get_restore_open(), "the page opens anyway");
    let bridge = ui.restore_bridge();
    assert!(bridge.get_has_notice());
    assert_eq!(bridge.get_notice().problem, RestoreProblem::ListingFailed);
    assert_eq!(bridge.get_rows().row_count(), 0);
}

#[test]
fn escape_goes_back() {
    let (_site, p) = paged_with_page();
    p.ui.key(Key::Escape);
    assert!(!p.ui.window.get_restore_open());
}

#[test]
fn escape_is_ignored_while_blocked() {
    let (_site, p) = paged_with_page();
    p.bridge().set_confirm_open(true);
    assert!(p.bridge().get_blocked());
    p.ui.key(Key::Escape);
    assert!(p.ui.window.get_restore_open());
}

#[test]
fn escape_works_again_after_a_dialog_closes() {
    let (_site, p) = paged_with_page();
    p.bridge().set_confirm_open(true);
    p.bridge().set_confirm_open(false);
    p.ui.key(Key::Escape);
    assert!(!p.ui.window.get_restore_open(), "the page took the focus back");
}

#[test]
fn the_back_button_goes_back() {
    let (_site, p) = paged_with_page();
    p.ui.element("Back", Button).invoke_accessible_default_action();
    assert!(!p.ui.window.get_restore_open());
}

#[test]
fn a_dismissed_notice_disappears() {
    let (_site, p) = paged_with_page();
    p.bridge().set_notice(RestoreNotice {
        problem: RestoreProblem::Failed,
        value: "".into(),
    });
    p.bridge().set_has_notice(true);
    p.ui.element("Dismiss", Button).invoke_accessible_default_action();
    assert!(!p.bridge().get_has_notice());
}

#[test]
fn each_row_has_its_restore_button() {
    let site = editing_site();
    let p = Paged::home(&site);
    for minutes in 0..3 {
        p.backup("before-save", minutes);
    }
    p.open_from_home();
    for stamp in ["2020-09-13 12:26 UTC", "2020-09-13 12:27 UTC", "2020-09-13 12:28 UTC"] {
        assert!(p.ui.has_element(&format!("Restore the backup of {stamp}"), Button), "{stamp}");
    }
}

#[test]
fn a_row_without_a_date_has_a_generic_button() {
    let site = editing_site();
    let p = Paged::home(&site);
    let backup = p.backup("before-save", 0);
    common::rewrite_manifest(&backup, "created", "yesterday");
    p.open_from_home();
    assert_eq!(p.rows()[0].stamp.as_str(), "");
    assert!(p.ui.has_element("Restore this backup", Button));
}

#[test]
fn no_bridge_string_holds_the_save_folder() {
    let site = editing_site();
    let p = Paged::home(&site);
    let backup = p.backup("before-save", 0);
    common::rewrite_manifest(&backup, "created", "yesterday");
    p.backup("pre-upgrade", 1);
    p.open_from_home();

    let folder = p.folder.to_string_lossy().into_owned();
    let mut strings = vec![
        p.bridge().get_game_title().to_string(),
        p.bridge().get_notice().value.to_string(),
        p.bridge().get_pending_stamp().to_string(),
    ];
    for row in p.rows() {
        strings.push(row.stamp.to_string());
        strings.push(row.file.to_string());
    }
    for text in strings {
        assert!(!text.contains(&folder), "a bridge string holds the save folder");
    }
}

#[test]
fn twenty_one_rows_fit_the_list() {
    let site = editing_site();
    let p = Paged::home(&site);
    let backups: Vec<_> = (0..20).map(|minutes| p.backup("before-save", minutes)).collect();
    // Creating a backup prunes to 20; a restore of the oldest keeps it beside
    // a new safety backup, which is how a game reaches 21. A copy of the
    // oldest folder under an older stamp builds the same list.
    let oldest = &backups[0].dir;
    let copy = oldest.parent().unwrap().join("20190101-000000");
    fs::create_dir(&copy).unwrap();
    for entry in fs::read_dir(oldest).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), copy.join(entry.file_name())).unwrap();
    }
    p.open_from_home();
    assert_eq!(p.rows().len(), 21);
}
