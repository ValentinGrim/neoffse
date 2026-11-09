//! The private slot files of any game, decoded. Failure messages carry the
//! label and a field path, never a value, a length or a name
//! (`docs/architecture.md#test-policy`).

use std::path::Path;

use neoffse_format_pr::container::decode;
use neoffse_format_pr::document::{Document, Segment};
use neoffse_format_pr::shared::SlotModel;
use neoffse_format_pr::slots::SLOT_FILES;

use super::private::tables;

/// The value of `result`, or a panic with `what` alone. The `Err` is dropped
/// because its text can quote save content.
#[track_caller]
pub(crate) fn must<T, E>(result: Result<T, E>, what: &str) -> T {
    result.unwrap_or_else(|_| panic!("{what}"))
}

/// The value of `option`, or a panic with `what`.
#[track_caller]
pub(crate) fn must_some<T>(option: Option<T>, what: &str) -> T {
    match option {
        Some(value) => value,
        None => panic!("{what}"),
    }
}

/// One private slot file.
pub(crate) struct RealSlot {
    /// The game folder (`ff1`, `ff2`, ...) plus the first 8 characters of the file name.
    pub label: String,
    /// The file name; a save file name holds no personal value.
    pub name: String,
    pub bytes: Vec<u8>,
    pub json: String,
}

impl RealSlot {
    /// This slot with other file `bytes` and the `json` they decode to: the
    /// result of an edit written and read back.
    pub(crate) fn with_bytes(&self, bytes: Vec<u8>, json: String) -> Self {
        Self {
            label: self.label.clone(),
            name: self.name.clone(),
            bytes,
            json,
        }
    }

    /// A model `M` over the decoded JSON.
    pub(crate) fn model<M: SlotModel>(&self) -> M {
        let Ok(document) = Document::parse(&self.json) else {
            panic!("{}: the decoded JSON does not parse", self.label)
        };
        M::new(document, tables::<M>())
    }
}

pub(super) fn label(game: &str, path: &Path) -> String {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    format!("{game} {}", name.chars().take(8).collect::<String>())
}

/// The private files of `game` (`ff1`, `ff2`, ...) named in `SLOT_FILES` that
/// hold a `userData` key. Any other file (the bestiary, the slot index, the
/// settings, a stray `desktop.ini`) is left out unread; a slot file that does
/// not decode panics. `None` after a skip report when no slot file is left.
pub(crate) fn slots_of(game: &str) -> Option<Vec<RealSlot>> {
    let files = super::save_files(game)?;
    let mut slots = Vec::new();
    for path in files {
        let label = label(game, &path);
        let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
        if !SLOT_FILES.iter().any(|slot| slot.name == name) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else { panic!("{label}: read failed") };
        let Ok(decoded) = decode(&bytes) else { panic!("{label}: decode failed") };
        let Ok(mut document) = Document::parse(&decoded.json) else {
            panic!("{label}: parse failed")
        };
        if document.raw(&[Segment::Key("userData")]).is_ok() {
            slots.push(RealSlot {
                label,
                name,
                bytes,
                json: decoded.json,
            });
        }
    }
    if slots.is_empty() {
        neoffse_testkit::skip(&format!("{game} holds no slot file"));
        return None;
    }
    Some(slots)
}

/// One private file, whatever it holds.
pub(crate) struct RealFile {
    /// The game folder (`ff1`, `ff2`, ...) plus the first 8 characters of the file name.
    pub label: String,
    /// The file name.
    pub name: String,
    pub bytes: Vec<u8>,
}
