//! The write pipeline on the real file system.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::cell::Cell;
use std::error::Error as _;
use std::fs;
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use common::{is_temp_name, snapshot, test_dir};
use neoffse_io::atomic::{FileOps, RealFileOps, write};
use neoffse_io::error::{IoError, Operation};

/// A hook that accepts every file.
fn accept() -> impl FnOnce(&Path) -> io::Result<()> {
    |_| Ok(())
}

/// A folder holding `slot.sav` with `original` and one sibling.
fn populated(name: &str) -> std::path::PathBuf {
    let dir = test_dir(name);
    fs::write(dir.join("slot.sav"), b"original").unwrap();
    fs::write(dir.join("sibling.sav"), b"sibling").unwrap();
    dir
}

#[test]
fn replaces_the_existing_target() {
    let dir = populated("replace");
    write(&dir.join("slot.sav"), b"new content", accept()).unwrap();
    let after = snapshot(&dir);
    assert_eq!(after.len(), 2);
    assert_eq!(after["slot.sav"], b"new content");
    assert_eq!(after["sibling.sav"], b"sibling");
}

#[test]
fn creates_a_missing_target() {
    let dir = test_dir("create");
    write(&dir.join("slot.sav"), b"fresh", accept()).unwrap();
    let after = snapshot(&dir);
    assert_eq!(after.len(), 1);
    assert_eq!(after["slot.sav"], b"fresh");
}

#[test]
fn writes_every_byte_value_and_empty_input() {
    let dir = populated("bytes");
    let target = dir.join("slot.sav");
    let all: Vec<u8> = (0..400).flat_map(|_| 0..=u8::MAX).collect();
    assert_eq!(all.len(), 102_400);
    write(&target, &all, accept()).unwrap();
    assert_eq!(fs::read(&target).unwrap(), all);

    write(&target, b"", accept()).unwrap();
    assert_eq!(fs::metadata(&target).unwrap().len(), 0);
}

#[test]
fn hook_reads_a_sibling_file_before_the_rename() {
    let dir = populated("hook-reads");
    let target = dir.join("slot.sav");
    let seen = Cell::new(false);
    write(&target, b"checked content", |temp: &Path| {
        assert_eq!(temp.parent(), target.parent());
        assert_ne!(temp, target);
        assert_eq!(fs::read(temp).unwrap(), b"checked content");
        assert_eq!(fs::read(&target).unwrap(), b"original");
        seen.set(true);
        Ok::<(), io::Error>(())
    })
    .unwrap();
    assert!(seen.get());
    assert_eq!(fs::read(&target).unwrap(), b"checked content");
}

#[test]
fn hook_errors_of_any_convertible_type_are_the_source() {
    let dir = populated("hook-errors");
    let target = dir.join("slot.sav");
    let before = snapshot(&dir);

    let text = write(&target, b"rejected", |_: &Path| Err::<(), &str>("text")).unwrap_err();
    assert!(matches!(text, IoError::VerificationFailed { .. }));
    assert_eq!(text.source().unwrap().to_string(), "text");

    let io = write(&target, b"rejected", |_: &Path| Err::<(), _>(io::Error::other("io-cause"))).unwrap_err();
    assert!(matches!(io, IoError::VerificationFailed { .. }));
    assert_eq!(io.source().unwrap().to_string(), "io-cause");
    assert_eq!(snapshot(&dir), before);
}

#[test]
fn failed_verification_keeps_the_original() {
    let dir = populated("verification");
    let before = snapshot(&dir);
    let result = write(&dir.join("slot.sav"), b"rejected", |_: &Path| Err::<(), _>(io::Error::other("no")));
    assert!(matches!(result, Err(IoError::VerificationFailed { .. })));
    assert_eq!(snapshot(&dir), before);
}

#[test]
fn panicking_hook_removes_the_temp_file() {
    let dir = populated("panic");
    let before = snapshot(&dir);
    let target = dir.join("slot.sav");
    let outcome = catch_unwind(AssertUnwindSafe(|| {
        let _ = write(&target, b"never lands", |_: &Path| -> io::Result<()> { panic!("hook panic") });
    }));
    assert!(outcome.is_err());
    assert_eq!(snapshot(&dir), before);
}

#[test]
fn missing_folder_is_an_error_and_is_not_created() {
    let absent = test_dir("missing-folder").join("absent");
    let error = write(&absent.join("slot.sav"), b"x", accept()).unwrap_err();
    assert!(matches!(
        &error,
        IoError::Io { operation: Operation::CreateTemp, path, .. } if *path == absent
    ));
    assert!(!absent.exists());
}

#[test]
fn target_without_a_folder_part_is_rejected() {
    let ran = Cell::new(false);
    let error = write(Path::new(""), b"x", |_: &Path| {
        ran.set(true);
        Ok::<(), io::Error>(())
    })
    .unwrap_err();
    match error {
        IoError::Io {
            operation: Operation::CreateTemp,
            source,
            ..
        } => {
            assert_eq!(source.kind(), io::ErrorKind::InvalidInput);
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(!ran.get());
}

#[test]
fn target_that_is_a_directory_is_left_alone() {
    let dir = test_dir("dir-target");
    let target = dir.join("slot.sav");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("inside.bin"), b"inside").unwrap();
    let before = (snapshot(&dir), snapshot(&target));

    let error = write(&target, b"x", accept()).unwrap_err();
    assert!(matches!(
        &error,
        IoError::Io { operation: Operation::Rename, path, .. } if *path == target
    ));
    assert_eq!((snapshot(&dir), snapshot(&target)), before);
}

#[test]
fn real_create_temp_places_unique_files_in_the_folder() {
    let dir = test_dir("create-temp");
    let (first_file, first) = RealFileOps.create_temp(&dir).unwrap();
    let (second_file, second) = RealFileOps.create_temp(&dir).unwrap();
    assert_ne!(first, second);
    for path in [&first, &second] {
        assert_eq!(path.parent(), Some(dir.as_path()));
        let name = path.file_name().unwrap().to_string_lossy();
        assert!(is_temp_name(&name), "{name}");
        assert_eq!(fs::metadata(path).unwrap().len(), 0);
    }
    drop((first_file, second_file));
    RealFileOps.remove_file(&first).unwrap();
    RealFileOps.remove_file(&second).unwrap();
    assert!(snapshot(&dir).is_empty());
}

#[cfg(unix)]
#[test]
fn real_sync_dir_reports_a_missing_folder() {
    let dir = test_dir("sync-dir");
    RealFileOps.sync_dir(&dir).unwrap();
    assert!(RealFileOps.sync_dir(&dir.join("absent")).is_err());
}

#[cfg(windows)]
#[test]
fn locked_target_fails_the_rename_and_keeps_the_original() {
    use std::os::windows::fs::OpenOptionsExt as _;

    let dir = populated("locked");
    let target = dir.join("slot.sav");
    let before = snapshot(&dir);
    let lock = fs::OpenOptions::new().read(true).share_mode(0).open(&target).unwrap();
    let error = write(&target, b"x", accept()).unwrap_err();
    assert!(matches!(
        &error,
        IoError::Io {
            operation: Operation::Rename,
            ..
        }
    ));
    drop(lock);
    assert_eq!(snapshot(&dir), before);
}

#[cfg(windows)]
#[test]
fn written_target_is_not_marked_temporary() {
    use std::os::windows::fs::MetadataExt as _;

    let dir = populated("not-temporary");
    let target = dir.join("slot.sav");
    write(&target, b"x", accept()).unwrap();
    assert_eq!(fs::metadata(&target).unwrap().file_attributes() & 0x100, 0);
}

#[cfg(unix)]
#[test]
fn existing_mode_is_kept() {
    use std::os::unix::fs::PermissionsExt as _;

    for mode in [0o644, 0o640, 0o444] {
        let dir = test_dir(&format!("mode-{mode:o}"));
        let target = dir.join("slot.sav");
        fs::write(&target, b"original").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(mode)).unwrap();
        write(&target, b"rewritten", accept()).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"rewritten");
        let kept = fs::metadata(&target).unwrap().permissions().mode() & 0o7777;
        assert_eq!(kept, mode, "mode {mode:o}");
    }
}

#[cfg(unix)]
#[test]
fn new_target_has_no_group_or_other_access() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = test_dir("new-mode");
    let target = dir.join("slot.sav");
    write(&target, b"fresh", accept()).unwrap();
    assert_eq!(fs::metadata(&target).unwrap().permissions().mode() & 0o077, 0);
}

#[test]
fn real_copy_permissions_ignores_a_missing_source() {
    let dir = test_dir("copy-permissions");
    let (file, path) = RealFileOps.create_temp(&dir).unwrap();
    let before = file.metadata().unwrap().permissions();
    RealFileOps.copy_permissions(&dir.join("absent"), &file).unwrap();
    assert_eq!(file.metadata().unwrap().permissions(), before);
    drop(file);
    RealFileOps.remove_file(&path).unwrap();
}
