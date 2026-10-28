//! A path made unreadable for the length of a test.

#![cfg(unix)]

use std::fs::{self, File, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// A path with mode `0o000`. Dropping it restores the mode it had, so the
/// temporary folder can be deleted on every exit.
pub(crate) struct Locked {
    path: PathBuf,
    previous: Permissions,
}

impl Drop for Locked {
    fn drop(&mut self) {
        // Nothing can recover a failed restore here; the temporary folder then stays in /tmp.
        let _ = fs::set_permissions(&self.path, self.previous.clone());
    }
}

/// Removes every right on `path`, a file or a folder. Returns `None` after
/// reporting a skip when the process ignores permissions (root), so the test
/// returns early.
pub(crate) fn lock(path: &Path) -> Option<Locked> {
    let previous = fs::metadata(path).unwrap().permissions();
    fs::set_permissions(path, Permissions::from_mode(0o000)).unwrap();
    let locked = Locked {
        path: path.to_path_buf(),
        previous,
    };
    let refused = if path.is_dir() {
        fs::read_dir(path).is_err()
    } else {
        File::open(path).is_err()
    };
    if refused {
        return Some(locked);
    }
    neoffse_testkit::skip("the process ignores file permissions (root)");
    None
}
