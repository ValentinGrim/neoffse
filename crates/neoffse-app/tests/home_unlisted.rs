//! A save folder the scan cannot list is flagged, not shown as empty, and the
//! skipped counter counts only the places discovery could not read.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{ID_1, Site, Ui, all_rows, scanned, slot_bytes, slot_file, steam_folder, steam_slot};
use i_slint_backend_testing::AccessibleRole::Text;

const FF1: usize = 0;
#[cfg(unix)]
const FF2: usize = 1;

#[cfg(unix)]
#[test]
fn an_unlistable_folder_is_not_reported_as_empty() {
    use common::{lock, slots_of};

    let site = Site::new();
    let folder = steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let Some(_locked) = lock(&folder) else { return };
    let home = scanned(&site);

    let rows = all_rows(&home);
    assert!(rows[0].unlisted);
    assert!(!rows[0].no_save);
    assert_eq!(slots_of(&rows[0]).len(), 0);
    assert_eq!(home.skipped_count(), 0);
}

#[cfg(unix)]
#[test]
fn only_the_unlistable_folder_of_two_is_flagged() {
    use common::{ID_2, items, lock};
    use neoffse_app::FolderRow;
    use slint::Model;

    let site = Site::new();
    let first = steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    steam_slot(&site, FF1, ID_2, slot_file(2), &slot_bytes(2));
    let Some(_locked) = lock(&first) else { return };
    let home = scanned(&site);

    let rows = all_rows(&home);
    let folders: Vec<FolderRow> = items(&rows[0].folders);
    assert_eq!(folders.len(), 2);
    assert!(folders[0].unlisted && folders[0].slots.row_count() == 0);
    assert!(!folders[1].unlisted && folders[1].slots.row_count() == 1);
    assert!(!rows[0].unlisted);
}

#[cfg(unix)]
#[test]
fn an_unlistable_folder_of_an_unsupported_game_is_flagged() {
    use common::lock;

    let site = Site::new();
    let folder = steam_slot(&site, FF2, ID_1, slot_file(1), &slot_bytes(1));
    let Some(_locked) = lock(&folder) else { return };
    let home = scanned(&site);

    let rows = all_rows(&home);
    assert!(rows[1].unlisted);
    assert!(!rows[1].no_save);
}

#[cfg(unix)]
#[test]
fn the_window_says_the_folder_could_not_be_read() {
    use common::lock;

    let site = Site::new();
    let folder = steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let Some(_locked) = lock(&folder) else { return };
    let ui = Ui::scanned(&site, None);

    assert!(ui.has_element("The save folder could not be read", Text));
    assert!(!ui.has_element("No save found", Text));
}

#[test]
fn an_empty_folder_still_reads_no_save_found() {
    let site = Site::new();
    steam_folder(&site, FF1, ID_1);
    let home = scanned(&site);
    let rows = all_rows(&home);
    assert!(rows[0].no_save && !rows[0].unlisted);

    let ui = Ui::scanned(&site, None);
    assert!(ui.has_element("No save found", Text));
    assert!(!ui.has_element("The save folder could not be read", Text));
}

#[test]
fn a_broken_library_list_is_the_one_thing_the_counter_counts() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let list = site.root().join("home/.local/share/Steam/steamapps/libraryfolders.vdf");
    std::fs::write(list, b"{{{").unwrap();
    let home = scanned(&site);
    assert_eq!(home.skipped_count(), 1);

    let ui = Ui::scanned(&site, None);
    assert!(ui.has_element("Some places could not be searched (1)", Text));
    assert!(!ui.has_element("Some files or folders could not be read (1)", Text));
}
