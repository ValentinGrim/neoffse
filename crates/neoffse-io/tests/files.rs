//! `files::list` and `files::read` on temporary folders.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::fs::{self, File};
use std::path::Path;

use neoffse_io::atomic::is_temp_name;
use neoffse_io::error::{IoError, Operation};
use neoffse_io::files::{FileHead, MAX_READ_LEN, list, read};

fn folder() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn names(files: &[FileHead]) -> Vec<&str> {
    files.iter().map(|file| file.name.as_str()).collect()
}

fn assert_read_error(error: &IoError, dir: &Path, name: &str) {
    match error {
        IoError::Io {
            operation: Operation::Read,
            path,
            source: _,
        } => assert_eq!(path, &dir.join(name)),
        other => panic!("expected Io(Read), got {other:?}"),
    }
}

#[test]
fn list_sorts_by_name_and_cuts_the_heads() {
    let dir = folder();
    fs::write(dir.path().join("b.sav"), b"bbbbbbbb").unwrap();
    fs::write(dir.path().join("C.sav"), b"CCCC").unwrap();
    fs::write(dir.path().join("a=+.sav"), b"0123456789").unwrap();

    let listing = list(dir.path(), 4).unwrap();
    assert_eq!(names(&listing.files), ["C.sav", "a=+.sav", "b.sav"]);
    let heads: Vec<(&[u8], u64)> = listing.files.iter().map(|file| (file.head.as_slice(), file.len)).collect();
    assert_eq!(heads, [(&b"CCCC"[..], 4), (&b"0123"[..], 10), (&b"bbbb"[..], 8)]);
    assert_eq!(listing.non_utf8, 0);
    assert!(listing.unreadable.is_empty());
}

#[test]
fn list_gives_a_short_file_whole_and_an_empty_file_an_empty_head() {
    let dir = folder();
    fs::write(dir.path().join("short"), b"abc").unwrap();
    fs::write(dir.path().join("empty"), b"").unwrap();

    let listing = list(dir.path(), 16).unwrap();
    assert_eq!(names(&listing.files), ["empty", "short"]);
    assert_eq!((listing.files[0].len, listing.files[0].head.as_slice()), (0, &b""[..]));
    assert_eq!((listing.files[1].len, listing.files[1].head.as_slice()), (3, &b"abc"[..]));
}

#[test]
fn list_with_head_len_zero_reads_no_byte() {
    let dir = folder();
    fs::write(dir.path().join("one"), b"abcdef").unwrap();
    let listing = list(dir.path(), 0).unwrap();
    assert_eq!(names(&listing.files), ["one"]);
    assert_eq!(listing.files[0].len, 6);
    assert_eq!(listing.files[0].head, Vec::<u8>::new());
}

#[test]
fn list_leaves_out_a_subfolder() {
    let dir = folder();
    fs::create_dir(dir.path().join("sub")).unwrap();
    fs::write(dir.path().join("sub").join("inner"), b"x").unwrap();
    fs::write(dir.path().join("file"), b"x").unwrap();
    let listing = list(dir.path(), 1).unwrap();
    assert_eq!(names(&listing.files), ["file"]);
    assert_eq!(listing.non_utf8, 0);
    assert!(listing.unreadable.is_empty());
}

#[cfg(unix)]
#[test]
fn list_leaves_out_symbolic_links() {
    let dir = folder();
    fs::write(dir.path().join("real"), b"x").unwrap();
    std::os::unix::fs::symlink(dir.path().join("real"), dir.path().join("link-to-file")).unwrap();
    std::os::unix::fs::symlink(dir.path(), dir.path().join("link-to-dir")).unwrap();
    std::os::unix::fs::symlink(dir.path().join("missing"), dir.path().join("dangling")).unwrap();
    let listing = list(dir.path(), 1).unwrap();
    assert_eq!(names(&listing.files), ["real"]);
    assert!(listing.unreadable.is_empty());
}

#[test]
fn list_leaves_out_the_temporary_files_of_a_write() {
    let dir = folder();
    fs::write(dir.path().join(".neoffse-AbC123.tmp"), b"half").unwrap();
    fs::write(dir.path().join(".neoffse-.tmp"), b"not a temp name").unwrap();
    fs::write(dir.path().join("slot"), b"x").unwrap();
    assert!(is_temp_name(".neoffse-AbC123.tmp"));
    let listing = list(dir.path(), 4).unwrap();
    assert_eq!(names(&listing.files), [".neoffse-.tmp", "slot"]);
}

#[cfg(unix)]
#[test]
fn list_counts_names_that_are_not_utf8() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt as _;

    let dir = folder();
    fs::write(dir.path().join(OsStr::from_bytes(b"bad-\xff-name")), b"x").unwrap();
    fs::write(dir.path().join(OsStr::from_bytes(b"\xfe")), b"x").unwrap();
    fs::create_dir(dir.path().join(OsStr::from_bytes(b"dir-\xff"))).unwrap();
    fs::write(dir.path().join("good"), b"x").unwrap();
    let listing = list(dir.path(), 1).unwrap();
    assert_eq!(names(&listing.files), ["good"]);
    assert_eq!(listing.non_utf8, 2);
    assert!(listing.unreadable.is_empty());
}

#[test]
fn list_of_a_missing_folder_is_a_list_dir_error() {
    let dir = folder();
    let missing = dir.path().join("missing");
    match list(&missing, 4).unwrap_err() {
        IoError::Io {
            operation: Operation::ListDir,
            path,
            source,
        } => {
            assert_eq!(path, missing);
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected Io(ListDir), got {other:?}"),
    }
}

#[test]
fn list_of_a_file_is_a_list_dir_error() {
    let dir = folder();
    let file = dir.path().join("file");
    fs::write(&file, b"x").unwrap();
    assert!(matches!(
        list(&file, 4).unwrap_err(),
        IoError::Io {
            operation: Operation::ListDir,
            ..
        }
    ));
}

#[cfg(unix)]
#[test]
fn list_reports_a_file_it_cannot_read_and_goes_on() {
    use std::error::Error as _;
    use std::os::unix::fs::PermissionsExt as _;

    let dir = folder();
    let locked = dir.path().join("locked");
    fs::write(&locked, b"secret").unwrap();
    fs::write(dir.path().join("open"), b"abc").unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    if File::open(&locked).is_ok() {
        // A superuser reads any file: the case cannot happen.
        return;
    }
    let listing = list(dir.path(), 4).unwrap();
    assert_eq!(names(&listing.files), ["open"]);
    assert_eq!(listing.unreadable.len(), 1);
    assert_read_error(&listing.unreadable[0], dir.path(), "locked");
    assert!(listing.unreadable[0].source().is_some());
}

#[test]
fn read_refuses_names_that_are_not_plain() {
    let dir = folder();
    for name in ["", ".", "..", "a/b", "a\\b", "c:", "a:b", "nul\0name", "../x", "/abs"] {
        match read(dir.path(), name).unwrap_err() {
            IoError::InvalidFileName { name: refused } => assert_eq!(refused, name),
            other => panic!("{name:?}: expected InvalidFileName, got {other:?}"),
        }
    }
}

#[test]
fn read_of_a_missing_file_is_a_read_error() {
    let dir = folder();
    let error = read(dir.path(), "missing").unwrap_err();
    assert_read_error(&error, dir.path(), "missing");
    match error {
        IoError::Io { source, .. } => assert_eq!(source.kind(), std::io::ErrorKind::NotFound),
        other => panic!("{other:?}"),
    }
}

#[test]
fn read_of_a_folder_is_a_read_error() {
    let dir = folder();
    fs::create_dir(dir.path().join("sub")).unwrap();
    let error = read(dir.path(), "sub").unwrap_err();
    assert_read_error(&error, dir.path(), "sub");
}

#[test]
fn read_returns_the_bytes_of_a_file() {
    let dir = folder();
    let bytes: Vec<u8> = (0..=255).cycle().take(70_000).collect();
    fs::write(dir.path().join("ookr=+"), &bytes).unwrap();
    fs::write(dir.path().join("empty"), b"").unwrap();
    assert_eq!(read(dir.path(), "ookr=+").unwrap(), bytes);
    assert_eq!(read(dir.path(), "empty").unwrap(), Vec::<u8>::new());
}

#[test]
fn read_accepts_a_file_of_exactly_the_limit_and_refuses_one_byte_more() {
    let dir = folder();
    let sparse = |name: &str, len: u64| {
        File::create(dir.path().join(name)).unwrap().set_len(len).unwrap();
    };
    sparse("limit", MAX_READ_LEN);
    sparse("over", MAX_READ_LEN + 1);

    assert_eq!(read(dir.path(), "limit").unwrap().len() as u64, MAX_READ_LEN);
    match read(dir.path(), "over").unwrap_err() {
        IoError::FileTooLarge { path, limit } => {
            assert_eq!(path, dir.path().join("over"));
            assert_eq!(limit, MAX_READ_LEN);
        }
        other => panic!("expected FileTooLarge, got {other:?}"),
    }
}
