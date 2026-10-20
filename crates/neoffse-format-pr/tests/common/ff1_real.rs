//! The private FF I slot files, decoded. Failure messages carry the label and
//! a field path, never a value, a length or a name
//! (`docs/architecture.md#test-policy`).

use std::path::Path;

use neoffse_format_pr::container::decode;
use neoffse_format_pr::document::{Document, Segment};
use neoffse_format_pr::ff1::{Ff1Model, GAME};

/// One private slot file.
pub(crate) struct RealSlot {
    /// `ff1` plus the first 8 characters of the file name.
    pub label: String,
    /// The file name; a save file name holds no personal value.
    pub name: String,
    pub bytes: Vec<u8>,
    pub json: String,
}

impl RealSlot {
    /// A model over the decoded JSON.
    pub(crate) fn model(&self) -> Ff1Model {
        let Ok(document) = Document::parse(&self.json) else {
            panic!("{}: the decoded JSON does not parse", self.label)
        };
        let Some(tables) = neoffse_data::tables_for(GAME) else {
            panic!("no FF I tables")
        };
        Ff1Model::new(document, tables)
    }
}

fn label(path: &Path) -> String {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    format!("ff1 {}", name.chars().take(8).collect::<String>())
}

/// The FF I private files that hold a `userData` key (the bestiary, the slot
/// index and the settings do not). `None` after a skip report when there are
/// none.
pub(crate) fn slots() -> Option<Vec<RealSlot>> {
    let files = super::save_files("ff1")?;
    let mut slots = Vec::new();
    for path in files {
        let label = label(&path);
        let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
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
        neoffse_testkit::skip("ff1 holds no slot file");
        return None;
    }
    Some(slots)
}
