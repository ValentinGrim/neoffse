//! The slot file a proof edits: the synthetic fixture, a respaced copy of it
//! or a private slot, all in memory.

use neoffse_core::{FormatError, SaveDocument, SaveFiles, SaveFormat};
use neoffse_format_pr::container::{self, Container};
use neoffse_format_pr::ff1::Ff1Format;

use super::ff1_format::{one_file, sav, slot_name};
use super::ff1_real::RealSlot;
use super::ff1_save::SlotData;
use super::json_tree::Tree;

/// One slot file to edit in memory.
pub(crate) struct Subject {
    pub label: String,
    pub name: String,
    pub bytes: Vec<u8>,
    pub json: String,
    pub container: Container,
    pub tree: Tree,
    /// A private save: failure messages carry no value.
    pub private: bool,
}

impl Subject {
    pub(crate) fn fixture(slot: &SlotData, container: Container) -> Subject {
        let json = slot.render();
        Subject {
            label: "fixture".to_owned(),
            name: slot_name(1).to_owned(),
            bytes: sav(&json, &container),
            tree: Tree::parse(&json),
            json,
            container,
            private: false,
        }
    }

    /// The same slot with a space after each colon and comma above `depth`
    /// (`Tree::spaced`), encoded under the same container.
    pub(crate) fn respaced(&self, depth: usize) -> Subject {
        let json = self.tree.spaced(depth);
        Subject {
            label: format!("{} spaced", self.label),
            name: self.name.clone(),
            bytes: sav(&json, &self.container),
            tree: Tree::parse(&json),
            json,
            container: self.container.clone(),
            private: self.private,
        }
    }

    pub(crate) fn real(slot: &RealSlot) -> Subject {
        let Ok(decoded) = container::decode(&slot.bytes) else {
            panic!("{}: decode failed", slot.label)
        };
        Subject {
            label: slot.label.clone(),
            name: slot.name.clone(),
            bytes: slot.bytes.clone(),
            tree: Tree::parse(&decoded.json),
            json: decoded.json,
            container: decoded.container,
            private: true,
        }
    }

    pub(crate) fn open(&self) -> Box<dyn SaveDocument> {
        open_files(&one_file(&self.name, self.bytes.clone())).unwrap_or_else(|error| {
            assert!(!self.private, "{}: open failed", self.label);
            panic!("{}: open failed: {error}", self.label)
        })
    }
}

/// Opens `files` through the trait object the app calls.
pub(crate) fn open_files(files: &SaveFiles) -> Result<Box<dyn SaveDocument>, FormatError> {
    let format: &dyn SaveFormat = &Ff1Format::new();
    format.open(files)
}
