//! Which file names of a private game folder count as saves. The helper is
//! pure, so the tests need no folder and no environment edit.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::ffi::OsStr;

use common::is_save_name;
use common::{STEAM_CLOUD_MARKER, ff1_format::slot_name};

#[test]
fn a_slot_file_name_passes() {
    for id in [1, 20, 21, 22] {
        assert!(is_save_name(OsStr::new(slot_name(id))), "slot {id}");
    }
}

#[test]
fn the_steam_cloud_marker_is_refused() {
    assert!(!is_save_name(OsStr::new("steam_autocloud.vdf")));
    assert!(!is_save_name(OsStr::new(STEAM_CLOUD_MARKER)));
}

#[test]
fn a_kde_folder_file_is_refused() {
    assert!(!is_save_name(OsStr::new(".directory")));
}

#[test]
fn any_dot_file_is_refused() {
    for name in [".", "..", ".hidden", ".DS_Store"] {
        assert!(!is_save_name(OsStr::new(name)), "{name}");
    }
}

#[test]
fn a_name_outside_the_game_naming_is_refused() {
    let short = &slot_name(1)[..43];
    let long = format!("{}A", slot_name(1));
    let odd = slot_name(1).replacen(char::from(slot_name(1).as_bytes()[0]), "-", 1);
    for name in ["save.bak", "desktop.ini", "README.txt", short, long.as_str(), odd.as_str()] {
        assert!(!is_save_name(OsStr::new(name)), "{name}");
    }
}
