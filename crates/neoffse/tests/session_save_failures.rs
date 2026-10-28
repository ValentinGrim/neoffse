//! `Session::save_with` under injected write failures: the folder keeps the
//! file, the backup exists and the session stays dirty.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::cell::Cell;
use std::fs::{self, File};
use std::io;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use common::{data_dir, folder_with, gil, gil_value, moment, no_process, open_session, set, slot_name};
use neoffse::core::{Edit, Value};
use neoffse::io::atomic::{FileOps, RealFileOps};
use neoffse::io::backup;
use neoffse::io::error::{IoError, Operation};
use neoffse::pr::container::{self, Container, MAX_PAYLOAD_LEN};
use neoffse::{Error, Registry, Running, Session};
use tempfile::TempDir;

#[derive(Debug, Clone, Copy)]
enum Fault {
    FlipOneByte,
    FailRename,
}

/// Delegates to `RealFileOps` and breaks one step of the write into the save
/// folder. The backup writes through the same ops and must succeed first, so
/// the fault arms only when the temporary file is created in the save folder.
struct Faulty {
    fault: Fault,
    save_dir: PathBuf,
    armed: Cell<bool>,
}

impl Faulty {
    fn new(fault: Fault, save_dir: &Path) -> Self {
        Self {
            fault,
            save_dir: save_dir.to_path_buf(),
            armed: Cell::new(false),
        }
    }
}

impl FileOps for Faulty {
    fn create_temp(&self, dir: &Path) -> io::Result<(File, PathBuf)> {
        self.armed.set(dir == self.save_dir);
        RealFileOps.create_temp(dir)
    }

    fn write_all(&self, file: &mut File, bytes: &[u8]) -> io::Result<()> {
        if self.armed.get() && matches!(self.fault, Fault::FlipOneByte) {
            let mut flipped = bytes.to_vec();
            let middle = flipped.len() / 2;
            flipped[middle] ^= 0x01;
            return file.write_all(&flipped);
        }
        RealFileOps.write_all(file, bytes)
    }

    fn copy_permissions(&self, from: &Path, file: &File) -> io::Result<()> {
        RealFileOps.copy_permissions(from, file)
    }

    fn sync_file(&self, file: &File) -> io::Result<()> {
        RealFileOps.sync_file(file)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        if self.armed.get() && matches!(self.fault, Fault::FailRename) {
            return Err(io::Error::other("injected rename failure"));
        }
        RealFileOps.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        RealFileOps.remove_file(path)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        RealFileOps.sync_dir(dir)
    }
}

struct Outcome {
    dir: TempDir,
    session: Session,
    before: Vec<u8>,
    error: Error,
    backups: usize,
    original_before: Vec<u8>,
}

fn run(fault: Fault) -> Outcome {
    let (dir, mut session) = open_session();
    let (_data_root, data) = data_dir();
    let name = slot_name(2);
    let before = fs::read(dir.path().join(name)).unwrap();
    let start = gil_value(&mut session);
    session.edit(&set(&gil(), start + 1)).unwrap();
    let original_before = session.original().get(name).unwrap().to_vec();
    let error = session
        .save_with(&Faulty::new(fault, dir.path()), no_process, moment(), &data, Running::Allow)
        .unwrap_err();
    let backups = backup::list(&data, "ff1-pr").unwrap().backups;
    assert_eq!(backups.len(), 1, "the backup runs before the write");
    assert_eq!(fs::read(backups[0].dir.join(name)).unwrap(), before);
    Outcome {
        dir,
        session,
        before,
        error,
        backups: backups.len(),
        original_before,
    }
}

fn assert_unchanged(outcome: &Outcome) {
    let name = slot_name(2);
    assert_eq!(fs::read(outcome.dir.path().join(name)).unwrap(), outcome.before);
    let left: Vec<_> = fs::read_dir(outcome.dir.path()).unwrap().collect();
    assert_eq!(left.len(), 1, "no temporary file stays behind");
    assert!(outcome.session.is_dirty());
    assert_eq!(outcome.session.original().get(name), Some(outcome.original_before.as_slice()));
    assert_eq!(outcome.backups, 1);
}

#[test]
fn a_corrupted_write_fails_verification_and_leaves_the_file() {
    let outcome = run(Fault::FlipOneByte);
    match &outcome.error {
        Error::Io {
            source: IoError::VerificationFailed { .. },
        } => {}
        other => panic!("expected VerificationFailed, got {other:?}"),
    }
    assert_unchanged(&outcome);
}

#[test]
fn a_failed_rename_leaves_the_file() {
    let outcome = run(Fault::FailRename);
    match &outcome.error {
        Error::Io {
            source: IoError::Io {
                operation: Operation::Rename, ..
            },
        } => {}
        other => panic!("expected an Io error on rename, got {other:?}"),
    }
    assert_unchanged(&outcome);
}

#[test]
fn a_session_saves_again_after_a_failed_save() {
    let mut outcome = run(Fault::FailRename);
    let (_data_root, data) = data_dir();
    outcome.session.save_with(&RealFileOps, no_process, moment(), &data, Running::Allow).unwrap();
    assert!(!outcome.session.is_dirty());
}

/// The synthetic slot with its `pictureData` string padded until the JSON is
/// exactly `MAX_PAYLOAD_LEN` bytes: it still opens, and any longer edit
/// makes the encoder refuse the payload.
fn slot_at_the_payload_limit() -> Vec<u8> {
    const PICTURE: &str = "UE5HLWludmVudGVk";
    let json = String::from_utf8(neoffse_testkit::read_fixture("pr/ff1-slot.json").unwrap()).unwrap();
    let marker = format!("\"pictureData\":\"{PICTURE}\"");
    assert!(json.contains(&marker));
    let pad = MAX_PAYLOAD_LEN - json.len() + PICTURE.len();
    let padded = json.replace(&marker, &format!("\"pictureData\":\"{}\"", "A".repeat(pad)));
    assert_eq!(padded.len(), MAX_PAYLOAD_LEN);
    container::encode(&padded, &Container::game()).unwrap()
}

#[test]
fn an_encode_failure_takes_no_backup() {
    let bytes = slot_at_the_payload_limit();
    let name = slot_name(2);
    let dir = folder_with(&[(name, &bytes)]);
    let (data_root, data) = data_dir();
    let mut session = Registry::new(neoffse::data::Language::En).open(dir.path(), name, None).unwrap();
    let start = gil_value(&mut session);
    let longer = Edit::set(gil(), Value::Int(start * 1000 + 999));
    session.edit_expert(&longer).unwrap();

    let error = session.save_with(&RealFileOps, no_process, moment(), &data, Running::Allow).unwrap_err();

    assert!(matches!(error, Error::Format { .. }), "expected Format, got {error:?}");
    assert_eq!(fs::read(dir.path().join(name)).unwrap(), bytes);
    assert_eq!(fs::read_dir(data_root.path()).unwrap().count(), 0, "an encode failure took a backup");
    assert!(session.is_dirty());
}
