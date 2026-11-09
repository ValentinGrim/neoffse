//! The slot file a proof edits: the synthetic fixture, a respaced copy of it
//! or a private slot, all in memory. The subject carries the format of its
//! game, so the same proof runs on FF I and FF II.

use std::panic::RefUnwindSafe;
use std::sync::Arc;

use neoffse_core::{FormatError, Game, SaveDocument, SaveFiles, SaveFormat};
use neoffse_format_pr::container::{self, Container};

use super::ff1_save::SlotData;
use super::json_tree::Tree;
use super::pr_files::{one_file, sav, slot_name};
use super::real_slots::{RealSlot, must, must_some};

/// One slot file to edit in memory.
pub(crate) struct Subject {
    /// The format that opens `bytes` and every encoding of it.
    /// `RefUnwindSafe` so a test can catch the panic of a failed proof.
    format: Arc<dyn SaveFormat + RefUnwindSafe>,
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
    /// The FF I synthetic slot, opened by `format`.
    pub(crate) fn fixture(format: impl SaveFormat + RefUnwindSafe + 'static, slot: &SlotData, container: Container) -> Subject {
        Subject::from_json(format, slot.render(), container)
    }

    /// A synthetic slot given as JSON text, opened by `format`.
    pub(crate) fn from_json(format: impl SaveFormat + RefUnwindSafe + 'static, json: String, container: Container) -> Subject {
        Subject {
            format: Arc::new(format),
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
            format: Arc::clone(&self.format),
            label: format!("{} spaced", self.label),
            name: self.name.clone(),
            bytes: sav(&json, &self.container),
            tree: Tree::parse(&json),
            json,
            container: self.container.clone(),
            private: self.private,
        }
    }

    pub(crate) fn real(format: impl SaveFormat + RefUnwindSafe + 'static, slot: &RealSlot) -> Subject {
        let Ok(decoded) = container::decode(&slot.bytes) else {
            panic!("{}: decode failed", slot.label)
        };
        Subject {
            format: Arc::new(format),
            label: slot.label.clone(),
            name: slot.name.clone(),
            bytes: slot.bytes.clone(),
            tree: Tree::parse(&decoded.json),
            json: decoded.json,
            container: decoded.container,
            private: true,
        }
    }

    /// The game of the format.
    pub(crate) fn game(&self) -> Game {
        self.format.game()
    }

    pub(crate) fn open(&self) -> Box<dyn SaveDocument> {
        self.reopen(&one_file(&self.name, self.bytes.clone())).unwrap_or_else(|error| {
            assert!(!self.private, "{}: open failed", self.label);
            panic!("{}: open failed: {error}", self.label)
        })
    }

    /// Opens `files` through the trait object the app calls.
    pub(crate) fn reopen(&self, files: &SaveFiles) -> Result<Box<dyn SaveDocument>, FormatError> {
        self.format.open(files)
    }

    /// The decoded text and container of `document` equal the subject's. The
    /// text check covers the tree: equal text parses to equal trees. The
    /// messages carry no value.
    pub(crate) fn assert_original(&self, document: &dyn SaveDocument, what: &str) {
        let label = &self.label;
        let files = must(document.encode(), &format!("{label} {what}: encode failed"));
        let written = must_some(files.get(&self.name), &format!("{label} {what}: the encoding lacks the file"));
        let after = must(container::decode(written), &format!("{label} {what}: decode failed"));
        assert!(after.json == self.json, "{label} {what}: the text changed");
        assert!(after.container == self.container, "{label} {what}: the container changed");
    }
}
