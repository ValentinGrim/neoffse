//! Restore under failure: a failed write leaves its file as it was, the
//! restore stops there, and the safety backup is the way back.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::cell::{Cell, RefCell};
use std::error::Error as _;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use common::backup::{BESTIARY, EDITED_BESTIARY, EDITED_SLOT, SLOT, T0, World, at, edit, game_dir, make, restore_at, tree, world};
use common::{INJECTED, Step, is_temp_name};
use neoffse_io::atomic::{FileOps, RealFileOps};
use neoffse_io::backup::{Backup, Restored, restore_with};
use neoffse_io::error::{IoError, Operation};

const SAFETY_STAMP: &str = "20261018-201503";

const FIVE_STEPS: [Step; 5] = [Step::CreateTemp, Step::WriteAll, Step::CopyPermissions, Step::SyncFile, Step::Rename];

/// A `FileOps` over `RealFileOps` that fails the `n`th call (1-based) of one
/// step, or writes `corrupt` in place of the bytes of one `write_all` call.
#[derive(Debug)]
struct Gate {
    fail: Option<(Step, usize)>,
    corrupt_write: Option<usize>,
    calls: RefCell<Vec<Step>>,
    fired: Cell<bool>,
    renamed: RefCell<Vec<PathBuf>>,
}

impl Gate {
    fn new(fail: Option<(Step, usize)>, corrupt_write: Option<usize>) -> Self {
        Self {
            fail,
            corrupt_write,
            calls: RefCell::default(),
            fired: Cell::new(false),
            renamed: RefCell::default(),
        }
    }

    /// Records the call and returns its number among the calls of `step`.
    fn enter(&self, step: Step) -> io::Result<usize> {
        let mut calls = self.calls.borrow_mut();
        calls.push(step);
        let n = calls.iter().filter(|called| **called == step).count();
        if self.fail == Some((step, n)) {
            self.fired.set(true);
            return Err(io::Error::other(INJECTED));
        }
        Ok(n)
    }

    fn fired(&self) -> bool {
        self.fired.get()
    }

    fn renamed(&self) -> Vec<PathBuf> {
        self.renamed.borrow().clone()
    }
}

impl FileOps for Gate {
    fn create_temp(&self, dir: &Path) -> io::Result<(File, PathBuf)> {
        self.enter(Step::CreateTemp)?;
        RealFileOps.create_temp(dir)
    }

    fn write_all(&self, file: &mut File, bytes: &[u8]) -> io::Result<()> {
        let n = self.enter(Step::WriteAll)?;
        if self.corrupt_write == Some(n) {
            self.fired.set(true);
            return RealFileOps.write_all(file, b"corrupt");
        }
        RealFileOps.write_all(file, bytes)
    }

    fn copy_permissions(&self, from: &Path, file: &File) -> io::Result<()> {
        self.enter(Step::CopyPermissions)?;
        RealFileOps.copy_permissions(from, file)
    }

    fn sync_file(&self, file: &File) -> io::Result<()> {
        self.enter(Step::SyncFile)?;
        RealFileOps.sync_file(file)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.enter(Step::Rename)?;
        self.renamed.borrow_mut().push(to.to_path_buf());
        RealFileOps.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.enter(Step::RemoveFile)?;
        RealFileOps.remove_file(path)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        self.enter(Step::SyncDir)?;
        RealFileOps.sync_dir(dir)
    }
}

/// A world with a backup of the old files, then both files edited.
fn setup(name: &str) -> (World, Backup) {
    let w = world(&format!("restore-{name}"));
    let backup = make(&w, "ff1-pr", T0);
    edit(&w);
    (w, backup)
}

fn run(gate: &Gate, w: &World, backup: &Backup) -> Result<Restored, IoError> {
    restore_with(gate, &w.data, "ff1-pr", backup, &w.source, at(T0 + 60))
}

fn assert_no_temp_file(w: &World) {
    for entry in fs::read_dir(&w.source).unwrap() {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        assert!(!is_temp_name(&name), "stray temporary file {name}");
    }
}

/// The parts of an `IoError::Io`.
fn io_parts(error: &IoError) -> (Operation, &Path) {
    match error {
        IoError::Io { operation, path, .. } => (*operation, path),
        other => panic!("not an Io error: {other:?}"),
    }
}

/// The fields of a `RestoreFailed`: written, remaining, safety, source.
fn failed_parts(error: IoError) -> (Vec<String>, Vec<String>, Option<Box<Backup>>, Box<IoError>) {
    match error {
        IoError::RestoreFailed {
            written,
            remaining,
            safety,
            source,
        } => (written, remaining, safety, source),
        other => panic!("not a RestoreFailed error: {other:?}"),
    }
}

fn operation_of(step: Step) -> Operation {
    match step {
        Step::CreateTemp => Operation::CreateTemp,
        Step::WriteAll | Step::CopyPermissions => Operation::Write,
        Step::SyncFile => Operation::Sync,
        Step::Rename => Operation::Rename,
        other => panic!("no operation for {other:?}"),
    }
}

#[test]
fn a_failed_safety_backup_writes_nothing() {
    for step in FIVE_STEPS {
        let (w, backup) = setup("a_failed_safety_backup_writes_nothing");
        let source_before = tree(&w.source);
        let games_before = tree(&game_dir(&w, "ff1-pr"));
        let gate = Gate::new(Some((step, 1)), None);

        let error = run(&gate, &w, &backup).unwrap_err();
        assert!(gate.fired(), "{step:?}");
        let (operation, path) = io_parts(&error);
        assert_eq!(operation, operation_of(step), "{step:?}");
        assert!(path.starts_with(game_dir(&w, "ff1-pr").join(SAFETY_STAMP)), "{step:?}");
        assert_eq!(tree(&w.source), source_before, "{step:?}");
        assert_eq!(tree(&game_dir(&w, "ff1-pr")), games_before, "{step:?}");
        assert_no_temp_file(&w);
    }
}

#[test]
fn a_failed_first_write_leaves_the_target_unchanged() {
    for step in FIVE_STEPS {
        let (w, backup) = setup("a_failed_first_write_leaves_the_target_unchanged");
        let source_before = tree(&w.source);
        let gate = Gate::new(Some((step, 2)), None);

        let error = run(&gate, &w, &backup).unwrap_err();
        assert!(gate.fired(), "{step:?}");
        let (written, remaining, safety, source) = failed_parts(error);
        assert_eq!(written, Vec::<String>::new(), "{step:?}");
        assert_eq!(remaining, [SLOT, BESTIARY], "{step:?}");
        let safety = safety.unwrap();
        assert_eq!(fs::read(safety.dir.join(SLOT)).unwrap(), EDITED_SLOT);
        assert_eq!(fs::read(safety.dir.join(BESTIARY)).unwrap(), EDITED_BESTIARY);
        let (operation, path) = io_parts(&source);
        assert_eq!(operation, operation_of(step), "{step:?}");
        let expected = if step == Step::CreateTemp { w.source.clone() } else { w.source.join(SLOT) };
        assert_eq!(path, expected, "{step:?}");
        assert_eq!(tree(&w.source), source_before, "{step:?}");
        assert_no_temp_file(&w);
    }
}

#[test]
fn a_failed_second_write_stops_and_reports() {
    let (w, backup) = setup("a_failed_second_write_stops_and_reports");
    let gate = Gate::new(Some((Step::Rename, 3)), None);

    let error = run(&gate, &w, &backup).unwrap_err();
    assert!(gate.fired());
    assert_eq!(error.to_string(), "restore stopped after 1 of 2 files");
    // `thiserror` returns the boxed field itself, so the downcast target is the `Box`.
    let cause = error.source().unwrap().downcast_ref::<Box<IoError>>().unwrap();
    assert!(matches!(
        &**cause,
        IoError::Io {
            operation: Operation::Rename,
            ..
        }
    ));
    assert!(!error.to_string().contains(INJECTED));
    let (written, remaining, safety, source) = failed_parts(error);
    assert_eq!(written, [SLOT]);
    assert_eq!(remaining, [BESTIARY]);
    assert!(safety.is_some());
    assert_eq!(io_parts(&source).1, w.source.join(BESTIARY));
    assert_eq!(fs::read(w.source.join(SLOT)).unwrap(), b"slot-one");
    assert_eq!(fs::read(w.source.join(BESTIARY)).unwrap(), EDITED_BESTIARY);
    assert_no_temp_file(&w);
}

#[test]
fn the_safety_backup_undoes_a_partial_restore() {
    let (w, backup) = setup("the_safety_backup_undoes_a_partial_restore");
    let gate = Gate::new(Some((Step::Rename, 3)), None);
    let (_, _, safety, _) = failed_parts(run(&gate, &w, &backup).unwrap_err());
    assert_eq!(fs::read(w.source.join(SLOT)).unwrap(), b"slot-one");

    restore_at(&w, &safety.unwrap(), T0 + 120).unwrap();
    assert_eq!(fs::read(w.source.join(SLOT)).unwrap(), EDITED_SLOT);
    assert_eq!(fs::read(w.source.join(BESTIARY)).unwrap(), EDITED_BESTIARY);
}

#[test]
fn a_failed_verification_keeps_the_original() {
    let (w, backup) = setup("a_failed_verification_keeps_the_original");
    let source_before = tree(&w.source);
    let gate = Gate::new(None, Some(2));

    let error = run(&gate, &w, &backup).unwrap_err();
    assert!(gate.fired());
    let (written, _, _, source) = failed_parts(error);
    assert_eq!(written, Vec::<String>::new());
    assert!(matches!(&*source, IoError::VerificationFailed { path, .. } if *path == w.source.join(SLOT)));
    assert_eq!(tree(&w.source), source_before);
    assert_no_temp_file(&w);
}

#[test]
fn a_failed_verification_of_the_second_file_stops_there() {
    let (w, backup) = setup("a_failed_verification_of_the_second_file_stops_there");
    let gate = Gate::new(None, Some(3));

    let error = run(&gate, &w, &backup).unwrap_err();
    assert!(gate.fired());
    let (written, remaining, _, source) = failed_parts(error);
    assert_eq!(written, [SLOT]);
    assert_eq!(remaining, [BESTIARY]);
    assert!(matches!(&*source, IoError::VerificationFailed { path, .. } if *path == w.source.join(BESTIARY)));
    assert_eq!(fs::read(w.source.join(SLOT)).unwrap(), b"slot-one");
    assert_eq!(fs::read(w.source.join(BESTIARY)).unwrap(), EDITED_BESTIARY);
    assert_no_temp_file(&w);
}

#[test]
fn the_safety_backup_is_written_before_any_target_file() {
    let (w, backup) = setup("the_safety_backup_is_written_before_any_target_file");
    let gate = Gate::new(None, None);

    run(&gate, &w, &backup).unwrap();
    let expected = [
        game_dir(&w, "ff1-pr").join(SAFETY_STAMP).join("backup.json"),
        w.source.join(SLOT),
        w.source.join(BESTIARY),
    ];
    assert_eq!(gate.renamed(), expected);
}

#[test]
fn a_folder_in_place_of_a_target_file_fails_the_safety_backup() {
    let (w, backup) = setup("a_folder_in_place_of_a_target_file_fails_the_safety_backup");
    fs::remove_file(w.source.join(SLOT)).unwrap();
    fs::create_dir(w.source.join(SLOT)).unwrap();
    let before = (tree(&w.source), tree(&game_dir(&w, "ff1-pr")));

    let error = restore_at(&w, &backup, T0 + 60).unwrap_err();
    let (operation, path) = io_parts(&error);
    assert_eq!(operation, if cfg!(unix) { Operation::Copy } else { Operation::Read });
    assert_eq!(path, w.source.join(SLOT));
    assert_eq!(fs::read(w.source.join(BESTIARY)).unwrap(), EDITED_BESTIARY);
    assert_eq!((tree(&w.source), tree(&game_dir(&w, "ff1-pr"))), before);
}
