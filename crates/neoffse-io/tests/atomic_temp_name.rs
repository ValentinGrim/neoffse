//! `atomic::is_temp_name` against the names the write creates and against
//! look-alikes.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::path::Path;

use common::{Probe, Step, args, hook_ok, test_dir};
use neoffse_io::atomic::{is_temp_name, write_with};

#[test]
fn a_name_the_write_creates_is_a_temp_name() {
    let dir = test_dir("temp_name_created");
    let target = dir.join("slot.sav");
    let (probe, log) = Probe::new(&[Step::SyncFile]);
    write_with(&probe, &target, b"bytes", hook_ok(&log)).unwrap_err();

    let created = args(&log, "create_temp");
    let name = Path::new(&created[1]).file_name().unwrap().to_str().unwrap().to_owned();
    assert!(name.starts_with(".neoffse-"), "{name}");
    assert!(is_temp_name(&name), "{name}");
}

#[test]
fn a_name_with_nothing_between_prefix_and_suffix_is_not_a_temp_name() {
    assert!(!is_temp_name(".neoffse-.tmp"));
    assert!(!is_temp_name(".neoffse-tmp"));
    assert!(!is_temp_name(".neoffse-"));
}

#[test]
fn the_shape_of_a_temp_name_is_exact() {
    assert!(is_temp_name(".neoffse-x.tmp"));
    assert!(is_temp_name(".neoffse-AbC123.tmp"));
    assert!(is_temp_name(".neoffse-.tmp.tmp"));
    for name in [
        "neoffse-x.tmp",
        ".neoffse-x.TMP",
        ".Neoffse-x.tmp",
        ".neoffse-x.tmp.bak",
        "x.neoffse-x.tmp",
        "",
        "ookrbATYovG3tEOXIH4HqWnsv8TrUlRWzM8AlCmW2mk=",
        "backup.json",
    ] {
        assert!(!is_temp_name(name), "{name}");
    }
}
