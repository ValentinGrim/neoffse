//! The `Gate` file-ops wrapper of the restore failure tests. The module sits
//! under `common` and inherits its `dead_code` allowance.

use std::cell::{Cell, RefCell};
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use neoffse_io::atomic::{FileOps, RealFileOps};

use super::{INJECTED, Step};

/// A `FileOps` over `RealFileOps` that fails the `n`th call (1-based) of one
/// step, or writes the bytes of one `write_all` call with the first byte
/// changed: same length, different content.
#[derive(Debug)]
pub(crate) struct Gate {
    fail: Option<(Step, usize)>,
    corrupt_write: Option<usize>,
    calls: RefCell<Vec<Step>>,
    fired: Cell<bool>,
    renamed: RefCell<Vec<PathBuf>>,
}

impl Gate {
    pub(crate) fn new(fail: Option<(Step, usize)>, corrupt_write: Option<usize>) -> Self {
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

    /// Whether the injected failure or the corrupt write happened.
    pub(crate) fn fired(&self) -> bool {
        self.fired.get()
    }

    /// The targets of the `rename` calls, in call order.
    pub(crate) fn renamed(&self) -> Vec<PathBuf> {
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
            let mut changed = bytes.to_vec();
            if let Some(first) = changed.first_mut() {
                *first ^= 1;
            }
            return RealFileOps.write_all(file, &changed);
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

    fn copy_file(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.enter(Step::CopyFile)?;
        RealFileOps.copy_file(from, to)
    }
}
