//! Helpers shared by the registry test crates. Each crate declares
//! `mod common;`: Cargo does not build this file as a test target.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::fs;
use std::path::Path;

use neoffse::pr::container::{self, Container};
use neoffse::pr::slots::SLOT_FILES;
use tempfile::TempDir;

/// A synthetic FF I slot in the game's container: BOM, base64, CRLF.
pub(crate) fn slot_bytes() -> Vec<u8> {
    let json = neoffse_testkit::read_fixture("pr/ff1-slot.json").unwrap();
    let json = String::from_utf8(json).unwrap();
    container::encode(&json, &Container::game()).unwrap()
}

/// The PC file name of the `index`th row of `SLOT_FILES`.
pub(crate) fn slot_name(index: usize) -> &'static str {
    SLOT_FILES[index].name
}

/// A temporary folder holding the given files. Never a save folder.
pub(crate) fn folder_with(files: &[(&str, &[u8])]) -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (name, bytes) in files {
        write_in(dir.path(), name, bytes);
    }
    dir
}

pub(crate) fn write_in(dir: &Path, name: &str, bytes: &[u8]) {
    fs::write(dir.join(name), bytes).unwrap();
}
