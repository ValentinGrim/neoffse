//! Helpers of the `Ff1Format` and `Ff1Document` tests: framing a text into
//! a container, naming a slot file and opening a document.

use neoffse_core::SaveFiles;
use neoffse_format_pr::container::{Container, encode};
use neoffse_format_pr::ff1::{Ff1Document, Ff1Format};
use neoffse_format_pr::slots::SLOT_FILES;

use super::ff1_save::SlotData;

/// `json` as a container file.
pub(crate) fn sav(json: &str, container: &Container) -> Vec<u8> {
    encode(json, container).unwrap()
}

/// A set holding the one file `name`.
pub(crate) fn one_file(name: &str, bytes: Vec<u8>) -> SaveFiles {
    let mut files = SaveFiles::new();
    files.push(name, bytes).unwrap();
    files
}

/// The PC file name of the slot whose JSON `id` is `id`.
pub(crate) fn slot_name(id: u32) -> &'static str {
    SLOT_FILES.iter().find(|file| file.id == id).map(|file| file.name).unwrap()
}

/// The standard synthetic slot under the name of slot 1.
pub(crate) fn standard_files(container: &Container) -> SaveFiles {
    one_file(slot_name(1), sav(&SlotData::standard().render(), container))
}

/// The document of `files`, opened by the default format.
pub(crate) fn open(files: &SaveFiles) -> Ff1Document {
    Ff1Format::new().open_document(files).unwrap()
}
