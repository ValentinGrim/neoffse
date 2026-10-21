//! The file dialog behind a trait, so tests never open one.

use std::path::PathBuf;

/// Asks the user for a file. Blocking: the app calls it on a worker thread.
pub trait FilePicker: Send + Sync {
    /// The chosen file, or `None` when the user cancels or no dialog is
    /// available.
    fn pick(&self) -> Option<PathBuf>;
}

/// The native dialog: the Windows common item dialog, or the XDG desktop
/// portal on Linux, through `rfd`.
///
/// It needs a desktop session and opens a window when called, so tests use a
/// fake [`FilePicker`]. It sets no filter, no title and no parent: Pixel
/// Remaster slot files have no extension.
#[derive(Debug, Clone, Copy, Default)]
pub struct NativePicker;

impl FilePicker for NativePicker {
    fn pick(&self) -> Option<PathBuf> {
        rfd::FileDialog::new().pick_file()
    }
}
