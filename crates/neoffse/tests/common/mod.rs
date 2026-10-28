//! Helpers shared by the registry test crates. Each crate declares
//! `mod common;`: Cargo does not build this file as a test target.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::fs;
use std::path::Path;

use neoffse::Session;
use neoffse::core::{Edit, FieldId, Game, SaveFiles, Value};
use neoffse::io::backup::Moment;
use neoffse::io::dirs::DataDir;
use neoffse::pr::container::{self, Container};
use neoffse::pr::ff1::names;
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

/// A session opened on the synthetic slot. Keep the folder alive as long as
/// the session: it holds the file the session read.
pub(crate) fn open_session() -> (TempDir, neoffse::Session) {
    let bytes = slot_bytes();
    let dir = folder_with(&[(slot_name(2), &bytes)]);
    let session = neoffse::Registry::new(neoffse::data::Language::En)
        .open(dir.path(), slot_name(2), None)
        .unwrap();
    (dir, session)
}

/// Each FF I slot file of the private saves, copied under its own name into
/// its own temporary folder. The folders delete the copies on drop. Returns
/// `None` after reporting why when the saves hold no slot file, so the test
/// returns early and says so.
pub(crate) fn private_slot_copies() -> Option<Vec<(String, TempDir)>> {
    let paths = neoffse_testkit::private_saves("ff1")?;
    let copies: Vec<(String, TempDir)> = paths
        .iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?;
            SLOT_FILES.iter().any(|file| file.name == name).then(|| {
                let dir = tempfile::tempdir().unwrap();
                fs::copy(path, dir.path().join(name)).unwrap();
                (name.to_owned(), dir)
            })
        })
        .collect();
    if copies.is_empty() {
        neoffse_testkit::skip("no FF I slot file in the private saves");
        return None;
    }
    Some(copies)
}

/// A probe that finds no running game.
pub(crate) fn no_process(_executables: &[&'static str]) -> Vec<&'static str> {
    Vec::new()
}

/// A fixed instant for the backup folder name.
pub(crate) fn moment() -> Moment {
    Moment::new(1_700_000_000, 0).unwrap()
}

/// An empty data folder, apart from the save folder. Keep the guard alive.
pub(crate) fn data_dir() -> (TempDir, DataDir) {
    let dir = tempfile::tempdir().unwrap();
    let data = DataDir::at(dir.path());
    (dir, data)
}

pub(crate) fn gil() -> FieldId {
    FieldId::new(names::GIL)
}

pub(crate) fn level() -> FieldId {
    FieldId::new(names::CHARACTER).with_id(1).with_name(names::LEVEL)
}

pub(crate) fn set(field: &FieldId, value: i64) -> Edit {
    Edit::set(field.clone(), Value::Int(value))
}

pub(crate) fn gil_value(session: &mut Session) -> i64 {
    let view = session.view().unwrap();
    let (_, field) = view.find(&gil()).expect("gil is in the view");
    let Value::Int(gil) = field.value else { panic!("gil is an integer") };
    gil
}

/// Opens `before` with the FF I format, applies the gil edit through
/// `apply_game`, and verifies the file `name` in `dir` against that
/// document. `verify` compares the whole decoded document, unmodeled data
/// included, so `Ok` proves that the save changed nothing else.
pub(crate) fn assert_gil_only_change(before: &SaveFiles, gil_after: i64, dir: &Path, name: &str) {
    let registry = neoffse::Registry::new(neoffse::data::Language::En);
    let format = registry.format(Game::Ff1Pr).expect("FF I has a format");
    let mut document = format.open(before).unwrap();
    document.apply_game(&set(&gil(), gil_after)).unwrap();
    let mut written = SaveFiles::new();
    written.push(name, fs::read(dir.join(name)).unwrap()).unwrap();
    document.verify(&written).unwrap();
}
