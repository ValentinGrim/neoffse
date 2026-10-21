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
fn a_dot_inside_a_name_passes() {
    assert!(is_save_name(OsStr::new("save.bak")));
}
