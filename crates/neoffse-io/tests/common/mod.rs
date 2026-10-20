//! Helpers that the `atomic_*` test crates share. Each crate declares
//! `mod common;`: Cargo does not build this file as a test target.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use neoffse_io::atomic::{FileOps, RealFileOps};

pub(crate) mod backup;
pub(crate) mod discovery;
pub(crate) mod gate;

/// The message of every error a `Probe` injects.
pub(crate) const INJECTED: &str = "injected-3c1d";

/// Returns a fresh, empty folder `atomic-<name>` under the Cargo target
/// temporary folder.
pub(crate) fn test_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("atomic-{name}"));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Every entry of `dir` by file name with its bytes. A directory is an
/// empty entry whose name ends in `/`; the helper does not recurse.
pub(crate) fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut entries = BTreeMap::new();
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().unwrap().is_dir() {
            entries.insert(format!("{name}/"), Vec::new());
        } else {
            entries.insert(name, fs::read(entry.path()).unwrap());
        }
    }
    entries
}

/// A step of the pipeline that a `Probe` can fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Step {
    CreateTemp,
    WriteAll,
    CopyPermissions,
    SyncFile,
    Rename,
    RemoveFile,
    SyncDir,
    CopyFile,
}

/// The call log a `Probe` and the hook share. An entry is the method name,
/// then its path arguments, joined by `|`.
pub(crate) type Log = Rc<RefCell<Vec<String>>>;

/// A `FileOps` that delegates to `RealFileOps` and fails the chosen steps.
/// A failing step records its call, then returns `INJECTED` without
/// touching the disk, except `WriteAll`, which first writes half of the
/// bytes.
#[derive(Debug)]
pub(crate) struct Probe {
    fail_at: Vec<Step>,
    log: Log,
}

impl Probe {
    /// A probe that fails `fail_at`, with the log it writes to.
    pub(crate) fn new(fail_at: &[Step]) -> (Self, Log) {
        let log = Log::default();
        (
            Self {
                fail_at: fail_at.to_vec(),
                log: Rc::clone(&log),
            },
            log,
        )
    }

    fn record(&self, entry: String) {
        self.log.borrow_mut().push(entry);
    }

    fn fails(&self, step: Step) -> bool {
        self.fail_at.contains(&step)
    }
}

fn injected() -> io::Error {
    io::Error::other(INJECTED)
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

impl FileOps for Probe {
    fn create_temp(&self, dir: &Path) -> io::Result<(File, PathBuf)> {
        if self.fails(Step::CreateTemp) {
            self.record(format!("create_temp|{}", text(dir)));
            return Err(injected());
        }
        let (file, path) = RealFileOps.create_temp(dir)?;
        self.record(format!("create_temp|{}|{}", text(dir), text(&path)));
        Ok((file, path))
    }

    fn write_all(&self, file: &mut File, bytes: &[u8]) -> io::Result<()> {
        self.record("write_all".to_owned());
        if self.fails(Step::WriteAll) {
            file.write_all(&bytes[..bytes.len() / 2])?;
            return Err(injected());
        }
        RealFileOps.write_all(file, bytes)
    }

    fn copy_permissions(&self, from: &Path, file: &File) -> io::Result<()> {
        self.record(format!("copy_permissions|{}", text(from)));
        if self.fails(Step::CopyPermissions) {
            return Err(injected());
        }
        RealFileOps.copy_permissions(from, file)
    }

    fn sync_file(&self, file: &File) -> io::Result<()> {
        self.record("sync_file".to_owned());
        if self.fails(Step::SyncFile) {
            return Err(injected());
        }
        RealFileOps.sync_file(file)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.record(format!("rename|{}|{}", text(from), text(to)));
        if self.fails(Step::Rename) {
            return Err(injected());
        }
        RealFileOps.rename(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        self.record(format!("remove_file|{}", text(path)));
        if self.fails(Step::RemoveFile) {
            return Err(injected());
        }
        RealFileOps.remove_file(path)
    }

    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        self.record(format!("sync_dir|{}", text(dir)));
        if self.fails(Step::SyncDir) {
            return Err(injected());
        }
        RealFileOps.sync_dir(dir)
    }
}

/// A hook that logs `hook` and succeeds.
pub(crate) fn hook_ok(log: &Log) -> impl FnOnce(&Path) -> io::Result<()> + use<> {
    let log = Rc::clone(log);
    move |_| {
        log.borrow_mut().push("hook".to_owned());
        Ok(())
    }
}

/// A hook that logs `hook` and fails with `hook-9b2e`.
pub(crate) fn hook_err(log: &Log) -> impl FnOnce(&Path) -> io::Result<()> + use<> {
    let log = Rc::clone(log);
    move |_| {
        log.borrow_mut().push("hook".to_owned());
        Err(io::Error::other("hook-9b2e"))
    }
}

/// The method names of the log, in call order.
pub(crate) fn names(log: &Log) -> Vec<String> {
    log.borrow().iter().map(|entry| entry.split('|').next().unwrap().to_owned()).collect()
}

/// The `|`-separated arguments of the first log entry named `name`.
pub(crate) fn args(log: &Log, name: &str) -> Vec<String> {
    let log = log.borrow();
    let entry = log
        .iter()
        .find(|entry| entry.split('|').next() == Some(name))
        .unwrap_or_else(|| panic!("no `{name}` call in {log:?}"));
    entry.split('|').skip(1).map(str::to_owned).collect()
}

/// Whether `name` has the shape of a stray temporary file of the pipeline.
pub(crate) fn is_temp_name(name: &str) -> bool {
    name.strip_prefix(".neoffse-").and_then(|rest| rest.strip_suffix(".tmp")).is_some()
}
