//! The write pipeline under injected failures: each failing step leaves the
//! folder as it was.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;
use std::error::Error as _;
use std::fs;
use std::path::{Path, PathBuf};

use common::{INJECTED, Probe, Step, args, hook_err, hook_ok, is_temp_name, names, snapshot, test_dir};
use neoffse_io::atomic::write_with;
use neoffse_io::error::{IoError, Operation};

const NEW: &[u8] = b"replacement bytes";

struct Run {
    dir: PathBuf,
    target: PathBuf,
    before: BTreeMap<String, Vec<u8>>,
    log: common::Log,
    result: Result<(), IoError>,
}

/// Writes `NEW` over a target holding `original`, next to one sibling file.
fn run(name: &str, fail_at: &[Step], hook_fails: bool) -> Run {
    let dir = test_dir(name);
    let target = dir.join("slot.sav");
    fs::write(&target, b"original").unwrap();
    fs::write(dir.join("sibling.sav"), b"sibling").unwrap();
    let before = snapshot(&dir);
    let (probe, log) = Probe::new(fail_at);
    let result = if hook_fails {
        write_with(&probe, &target, NEW, hook_err(&log))
    } else {
        write_with(&probe, &target, NEW, hook_ok(&log))
    };
    Run {
        dir,
        target,
        before,
        log,
        result,
    }
}

fn io_parts(error: &IoError) -> (Operation, &Path, &std::io::Error) {
    match error {
        IoError::Io { operation, path, source } => (*operation, path, source),
        other => panic!("expected IoError::Io, got {other:?}"),
    }
}

fn assert_injected(error: &IoError) {
    let source = error.source().expect("an Io error keeps its cause");
    let cause = source.downcast_ref::<std::io::Error>().expect("the cause is an io::Error");
    assert_eq!(cause.to_string(), INJECTED);
}

fn assert_failed_io(run: &Run, operation: Operation, path: &Path, log: &[&str]) {
    let error = run.result.as_ref().unwrap_err();
    let (got_operation, got_path, _) = io_parts(error);
    assert_eq!(got_operation, operation);
    assert_eq!(got_path, path);
    assert_injected(error);
    assert_eq!(names(&run.log), log);
    assert_eq!(snapshot(&run.dir), run.before);
}

#[test]
fn steps_run_in_the_documented_order() {
    let run = run("order", &[], false);
    run.result.as_ref().unwrap();
    assert_eq!(
        names(&run.log),
        ["create_temp", "write_all", "copy_permissions", "sync_file", "hook", "rename", "sync_dir"]
    );
    let target = run.target.to_string_lossy();
    let dir = run.dir.to_string_lossy();
    assert_eq!(args(&run.log, "create_temp")[0], dir);
    assert_eq!(args(&run.log, "copy_permissions"), [target.as_ref()]);
    let rename = args(&run.log, "rename");
    assert_eq!(rename[0], args(&run.log, "create_temp")[1]);
    assert_eq!(rename[1], target);
    assert_eq!(args(&run.log, "sync_dir"), [dir.as_ref()]);
    assert_eq!(fs::read(&run.target).unwrap(), NEW);
}

#[test]
fn create_failure_leaves_the_folder_unchanged() {
    let run = run("create-failure", &[Step::CreateTemp], false);
    assert_failed_io(&run, Operation::CreateTemp, &run.dir, &["create_temp"]);
}

#[test]
fn write_failure_removes_the_torn_temp_file() {
    let run = run("write-failure", &[Step::WriteAll], false);
    assert_failed_io(&run, Operation::Write, &run.target, &["create_temp", "write_all", "remove_file"]);
    assert_eq!(args(&run.log, "remove_file")[0], args(&run.log, "create_temp")[1]);
}

#[test]
fn sync_failure_removes_the_temp_file() {
    let run = run("sync-failure", &[Step::SyncFile], false);
    assert_failed_io(
        &run,
        Operation::Sync,
        &run.target,
        &["create_temp", "write_all", "copy_permissions", "sync_file", "remove_file"],
    );
}

#[test]
fn permissions_failure_removes_the_temp_file() {
    let run = run("permissions-failure", &[Step::CopyPermissions], false);
    assert_failed_io(
        &run,
        Operation::Write,
        &run.target,
        &["create_temp", "write_all", "copy_permissions", "remove_file"],
    );
}

#[test]
fn verification_failure_removes_the_temp_file() {
    let run = run("verification-failure", &[], true);
    let error = run.result.as_ref().unwrap_err();
    assert!(matches!(error, IoError::VerificationFailed { path, .. } if *path == run.target));
    let cause = error.source().unwrap().downcast_ref::<std::io::Error>().unwrap();
    assert_eq!(cause.to_string(), "hook-9b2e");
    assert!(!error.to_string().contains("hook-9b2e"));
    assert_eq!(
        names(&run.log),
        ["create_temp", "write_all", "copy_permissions", "sync_file", "hook", "remove_file"]
    );
    assert_eq!(snapshot(&run.dir), run.before);
}

#[test]
fn rename_failure_removes_the_temp_file() {
    let run = run("rename-failure", &[Step::Rename], false);
    assert_failed_io(
        &run,
        Operation::Rename,
        &run.target,
        &["create_temp", "write_all", "copy_permissions", "sync_file", "hook", "rename", "remove_file"],
    );
}

#[test]
fn failed_cleanup_does_not_mask_the_step_error() {
    let run = run("cleanup-failure", &[Step::Rename, Step::RemoveFile], false);
    let error = run.result.as_ref().unwrap_err();
    let (operation, path, _) = io_parts(error);
    assert_eq!(operation, Operation::Rename);
    assert_eq!(path, run.target);
    assert_injected(error);

    let mut after = snapshot(&run.dir);
    assert_eq!(after.remove("slot.sav").unwrap(), b"original");
    assert_eq!(after.remove("sibling.sav").unwrap(), b"sibling");
    assert_eq!(after.len(), 1, "the failed removal leaves one file");
    let stray = after.keys().next().unwrap();
    assert!(is_temp_name(stray), "{stray}");
}

#[test]
fn sync_dir_failure_does_not_fail_the_write() {
    let run = run("sync-dir-failure", &[Step::SyncDir], false);
    run.result.as_ref().unwrap();
    assert_eq!(fs::read(&run.target).unwrap(), NEW);
}

#[test]
fn failure_without_an_original_creates_no_target() {
    for (name, step) in [("no-original-write", Step::WriteAll), ("no-original-rename", Step::Rename)] {
        let dir = test_dir(name);
        let target = dir.join("slot.sav");
        let (probe, log) = Probe::new(&[step]);
        let result = write_with(&probe, &target, NEW, hook_ok(&log));
        assert!(result.is_err());
        assert!(snapshot(&dir).is_empty(), "{name} left a file");
    }
}

#[test]
fn bare_file_name_uses_the_current_folder() {
    let (probe, log) = Probe::new(&[Step::CreateTemp]);
    let result = write_with(&probe, Path::new("bare.sav"), NEW, hook_ok(&log));
    let error = result.unwrap_err();
    let (operation, path, _) = io_parts(&error);
    assert_eq!(operation, Operation::CreateTemp);
    assert_eq!(path, Path::new("."));
    assert_eq!(args(&log, "create_temp"), ["."]);
}

#[test]
fn a_step_error_keeps_its_cause() {
    for step in [Step::CreateTemp, Step::WriteAll, Step::SyncFile, Step::CopyPermissions, Step::Rename] {
        let run = run("cause", &[step], false);
        assert_injected(run.result.as_ref().unwrap_err());
    }
}
