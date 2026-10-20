//! The write pipeline when the target is a symbolic link.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::error::Error as _;
use std::fs;
use std::io::{self, ErrorKind};
use std::os::unix::fs::{PermissionsExt as _, symlink};
use std::path::Path;

use common::{is_temp_name, test_dir};
use neoffse_io::atomic::write;
use neoffse_io::error::{IoError, Operation};

fn accept() -> impl FnOnce(&Path) -> io::Result<()> {
    |_| Ok(())
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn a_link_at_the_target_is_replaced_by_a_regular_file() {
    let dir = test_dir("link-replaced");
    let (real, slot) = (dir.join("real.sav"), dir.join("slot.sav"));
    fs::write(&real, b"linked").unwrap();
    fs::set_permissions(&real, fs::Permissions::from_mode(0o640)).unwrap();
    symlink("real.sav", &slot).unwrap();

    write(&slot, b"new", accept()).unwrap();

    let kind = fs::symlink_metadata(&slot).unwrap().file_type();
    assert!(kind.is_file() && !kind.is_symlink(), "the target is still a link");
    assert_eq!(fs::read(&slot).unwrap(), b"new");
    assert_eq!(mode(&slot), 0o640);
    assert_eq!(fs::read(&real).unwrap(), b"linked");
    assert_eq!(mode(&real), 0o640);
}

#[test]
fn a_looping_link_at_the_target_fails_copy_permissions_and_stays() {
    let dir = test_dir("link-loop");
    let target = dir.join("loop.sav");
    symlink("loop.sav", &target).unwrap();

    let error = write(&target, b"new", accept()).unwrap_err();

    let IoError::Io { operation, path, source } = &error else {
        panic!("unexpected error {error:?}");
    };
    assert_eq!(*operation, Operation::Write);
    assert_eq!(*path, target);
    assert_ne!(source.kind(), ErrorKind::NotFound);
    assert!(error.source().is_some());
    let names: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["loop.sav"], "stray files: {names:?}");
    assert!(!names.iter().any(|name| is_temp_name(name)));
    assert_eq!(fs::read_link(&target).unwrap(), Path::new("loop.sav"));
}
