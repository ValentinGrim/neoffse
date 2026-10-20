//! The two traits a save format implements: [`SaveFormat`] detects and opens
//! files, [`SaveDocument`] edits, encodes and verifies them.
//!
//! The registry, the edit history, the expert-mode setting and the save
//! pipeline that joins `encode`, the atomic write and `verify` live in the
//! facade crate (`docs/architecture.md#save-safety`).

use crate::edit::{ChangeSet, Edit, GameChange};
use crate::error::{EditError, FormatError};
use crate::field::{FieldId, RowId};
use crate::files::{Detection, FileProbe, SaveFiles};
use crate::game::Game;
use crate::kinds::{CapPolicy, Capability};
use crate::view::{SaveSummary, SaveView};

/// A save format for one game: it detects files and opens them.
pub trait SaveFormat: Send + Sync {
    /// The game this format reads and writes.
    fn game(&self) -> Game;

    /// Claims a file by its name and first bytes; `None` when the file is not
    /// this format's.
    ///
    /// The function is pure: it reads the probe and nothing else. It never
    /// claims a leftover of the atomic write of `neoffse-io`, whose
    /// temporary files are called `.neoffse-<random>.tmp`. The facade also
    /// skips such names before it calls `detect`; a format with fixed file
    /// names matches the name first.
    fn detect(&self, probe: &FileProbe<'_>) -> Option<Detection>;

    /// Reads the files of one save into a document.
    ///
    /// The format copies what it needs from `files` and the document keeps no
    /// borrow. A game the files do not fit is a `Decode` error, not a panic.
    ///
    /// # Errors
    ///
    /// `MissingFile` when a file the format needs is absent, `Decode` when it
    /// cannot read them.
    fn open(&self, files: &SaveFiles) -> Result<Box<dyn SaveDocument>, FormatError>;
}

/// An open save: the original data plus every edit applied to it.
///
/// The trait is dyn-compatible and requires `Send`, so the app moves a
/// document to a worker thread (`docs/architecture.md#ui-architecture`). It
/// has no `Debug` supertrait: a `Debug` print of a document cannot leak a
/// save.
pub trait SaveDocument: Send {
    /// The game of the save.
    fn game(&self) -> Game;

    /// The sections the editor offers. Every `Section::capability` of the
    /// view appears in the slice.
    fn capabilities(&self) -> &[Capability];

    /// What the Home screen shows. Takes `&mut self` because a format may
    /// parse lazily; no value changes.
    ///
    /// # Errors
    ///
    /// `Decode` when the lazy parse fails.
    fn summary(&mut self) -> Result<SaveSummary, FormatError>;

    /// The editable content, one section per capability. Takes `&mut self`
    /// because a format may parse lazily; no value changes.
    ///
    /// # Errors
    ///
    /// `Decode` when the lazy parse fails.
    fn view(&mut self) -> Result<SaveView, FormatError>;

    /// Applies one edit under `policy` and returns the inverse.
    ///
    /// By kind of edit: `Set` sets the field and returns a `Set` of the
    /// previous value. `Insert` adds the row at `position` (`None`: at the
    /// end) and returns `Remove { row }`. `Remove` deletes the row and
    /// returns an `Insert` with `position: Some(old index)` and every
    /// modeled field in `values`, so the inverse restores the row at its
    /// place and the order of the collection. A format that keeps data a row
    /// holds beyond its modeled fields restores that data too.
    ///
    /// An edit either succeeds or returns an `EditError` and leaves the
    /// document as it was, rows in the same order. A format validates with
    /// `FieldMeta::check`, `CollectionMeta::check_insert` and
    /// `CollectionMeta::check_remove` where it can, and raises
    /// `UnknownField`, `UnknownId`, `DuplicateRow`, `MissingRow`,
    /// `NotACollection`, `PositionOutOfRange` and `Format` itself.
    ///
    /// `CapPolicy::Enforce` checks the storage bounds and the game cap,
    /// `CapPolicy::StorageOnly` the storage bounds. The facade passes
    /// `Enforce` for a new edit unless the user turned expert mode on, and
    /// `StorageOnly` for an inverse, a rollback and a redo. A document
    /// therefore applies an inverse whatever value it restores, including one
    /// above the cap that the user set in expert mode. The document keeps no
    /// policy of its own: a setting inside the document could change between
    /// an edit and its undo.
    ///
    /// # Errors
    ///
    /// The `EditError` that refuses the edit.
    fn apply(&mut self, edit: &Edit, policy: CapPolicy) -> Result<Edit, EditError>;

    /// Applies a change set under `policy`, all or none, and returns the
    /// inverse set.
    ///
    /// The provided body calls [`ChangeSet::apply_with`]: forward edits get
    /// `policy`, the rollback gets `StorageOnly`. A format overrides it only
    /// for speed.
    ///
    /// # Errors
    ///
    /// The error of the first edit that fails, or `RollbackFailed`.
    fn apply_all(&mut self, changes: &ChangeSet, policy: CapPolicy) -> Result<ChangeSet, EditError> {
        changes.apply_with(policy, |edit, policy| self.apply(edit, policy))
    }

    /// Applies `edit` the way the game would and returns what changed.
    ///
    /// The provided body has no game rule beyond the caps: it applies `edit`
    /// under `CapPolicy::Enforce` and returns it as `forward` with its inverse
    /// as `inverse`. A format with game rules overrides it and may expand one
    /// edit into several plain edits. `forward` and `inverse` apply through
    /// [`apply_all`](Self::apply_all) under `CapPolicy::StorageOnly`. The UI
    /// calls `apply_game` for a new edit in the default mode and `apply` with
    /// its own policy in expert mode.
    ///
    /// # Errors
    ///
    /// The error of [`apply`](Self::apply) under `Enforce`, plus the game
    /// rules of the format. A failed call leaves the document as it was.
    fn apply_game(&mut self, edit: &Edit) -> Result<GameChange, EditError> {
        let inverse = self.apply(edit, CapPolicy::Enforce)?;
        Ok(GameChange::new(ChangeSet::from_iter([edit.clone()]), ChangeSet::from_iter([inverse])))
    }

    /// True when the game interface refuses to remove `row`, whatever the
    /// values. The UI greys the control before the click. The default is
    /// `false`. Takes `&mut self` because a format may parse lazily; no value
    /// changes.
    ///
    /// # Errors
    ///
    /// The format's error for a path or a row it cannot resolve.
    fn row_locked(&mut self, row: &RowId) -> Result<bool, EditError> {
        let _ = row;
        Ok(false)
    }

    /// True when the game interface refuses every `Set` of `field`, whatever
    /// the value. The UI greys the control before the click. The default is
    /// `false`.
    ///
    /// # Errors
    ///
    /// The format's error for a path that names no declared field.
    fn field_locked(&self, field: &FieldId) -> Result<bool, EditError> {
        let _ = field;
        Ok(false)
    }

    /// Encodes the document into the files to write, under the names `open`
    /// received. Changes nothing in the document.
    ///
    /// # Errors
    ///
    /// `Encode` when the format cannot write the document.
    fn encode(&self) -> Result<SaveFiles, FormatError>;

    /// Decodes `files` and compares the result with this document.
    ///
    /// The decode starts from the bytes alone, never from a cache, and the
    /// comparison covers the whole in-memory document, including data the
    /// view omits, fields no widget covers and the order of collections.
    /// `Ok` means the same document. The facade calls it from the hook of
    /// `neoffse_io::atomic::write`, with the temporary file's bytes read back
    /// from disk under the save file's logical name
    /// (`docs/architecture.md#save-safety`, steps 5 and 6). Changes nothing
    /// in the document.
    ///
    /// # Errors
    ///
    /// `VerifyMismatch` naming the first file that differs, `MissingFile` for
    /// an absent file, `Decode` for an unreadable one.
    fn verify(&self, files: &SaveFiles) -> Result<(), FormatError>;
}
