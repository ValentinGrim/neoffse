//! Helpers of the `Ff1Format` and `Ff1Document` tests: the standard synthetic
//! slot as files and the document opened from them.

use neoffse_core::SaveFiles;
use neoffse_format_pr::container::Container;
use neoffse_format_pr::ff1::{Ff1Document, Ff1Format};

use super::ff1_save::SlotData;
use super::pr_files::{one_file, sav, slot_name};

/// The standard synthetic slot under the name of slot 1.
pub(crate) fn standard_files(container: &Container) -> SaveFiles {
    one_file(slot_name(1), sav(&SlotData::standard().render(), container))
}

/// The document of `files`, opened by the default format.
pub(crate) fn open(files: &SaveFiles) -> Ff1Document {
    Ff1Format::new().open_document(files).unwrap()
}
