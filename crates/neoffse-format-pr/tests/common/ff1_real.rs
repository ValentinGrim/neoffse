//! The private FF I slot files, decoded. Failure messages carry the label and
//! a field path, never a value, a length or a name
//! (`docs/architecture.md#test-policy`).

use super::real_slots::{RealFile, RealSlot, label, slots_of};

/// The FF I private files named in `SLOT_FILES` that hold a `userData` key.
/// Any other file (the bestiary, the slot index, the settings, a stray
/// `desktop.ini`) is left out unread; a slot file that does not decode panics.
/// `None` after a skip report when no slot file is left.
pub(crate) fn slots() -> Option<Vec<RealSlot>> {
    slots_of("ff1")
}

/// Every private FF I file except the Steam marker: the slots, the bestiary,
/// the slot index and the settings. `None` after a skip report when there
/// are none.
pub(crate) fn files() -> Option<Vec<RealFile>> {
    let paths = super::save_files("ff1")?;
    let mut files = Vec::new();
    for path in paths {
        let label = label("ff1", &path);
        let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        let Ok(bytes) = std::fs::read(&path) else { panic!("{label}: read failed") };
        files.push(RealFile { label, name, bytes });
    }
    Some(files)
}
