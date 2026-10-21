//! One opened save and its edit history.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use neoffse_core::{
    CapPolicy, Capability, ChangeSet, Edit, EditError, FieldId, FormatError, Game, GameChange, RowId, SaveDocument, SaveFiles, SaveFormat, SaveSummary,
    SaveView,
};

use crate::error::Error;

/// One opened save and its edit history.
///
/// [`Registry::open`](crate::Registry::open) builds a session. The session
/// owns the document: every change goes through [`Session::edit`] or
/// [`Session::edit_expert`], so the history sees it. Undo and redo replay
/// the recorded changes. The dirty state counts the distance to a clean
/// point, the state of the files as read. [`Session::discard`] reopens the
/// document from the bytes read, held in memory, and touches no file.
///
/// `Session` is `Send` and not `Sync`: a caller that shares it across
/// threads wraps it in a lock.
///
/// ```no_run
/// use std::path::Path;
/// use std::sync::{Arc, Mutex};
/// use std::thread;
///
/// use neoffse::Registry;
/// use neoffse::data::Language;
///
/// # fn main() -> Result<(), neoffse::Error> {
/// let registry = Registry::new(Language::En);
/// let session = registry.open(Path::new("saves"), "slot.sav", None)?;
/// let session = Arc::new(Mutex::new(session));
///
/// let worker = Arc::clone(&session);
/// let handle = thread::spawn(move || {
///     let Ok(mut session) = worker.lock() else { return None };
///     session.view().ok()
/// });
/// if let Ok(Some(view)) = handle.join() {
///     println!("{} sections", view.sections.len());
/// }
/// # Ok(())
/// # }
/// ```
///
/// After [`EditError::RollbackFailed`] from an edit, undo or redo, the
/// document state is unknown: call [`Session::discard`].
///
/// `Debug` shows the game id, the file name, the history lengths and the
/// dirty state, never a byte or a path. [`Session::name`] and `Debug` carry
/// the file name, a personal value: never log them raw.
pub struct Session {
    format: Arc<dyn SaveFormat>,
    document: Box<dyn SaveDocument>,
    game: Game,
    dir: PathBuf,
    name: String,
    original: SaveFiles,
    undo: Vec<GameChange>,
    redo: Vec<GameChange>,
    /// The clean point is `base` entries deep in the undo stack. `extra`
    /// counts the entries lost when an edit dropped a redo branch that held
    /// the clean point: the stack length fell below `base`, so `base` follows
    /// it and `extra` keeps the difference. The distance to the clean point
    /// is `extra + |undo.len() - base|`: undoing back to `base` is clean,
    /// and after a fork the session stays dirty until the next save resets
    /// both counters.
    base: usize,
    extra: usize,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Session")
            .field("game", &self.game.id())
            .field("name", &self.name)
            .field("undo", &self.undo.len())
            .field("redo", &self.redo.len())
            .field("dirty", &self.is_dirty())
            .finish_non_exhaustive()
    }
}

impl Session {
    /// Starts the session of a document the registry just opened: empty
    /// history, clean point at the start.
    pub(crate) fn opened(format: Arc<dyn SaveFormat>, document: Box<dyn SaveDocument>, game: Game, dir: PathBuf, name: String, original: SaveFiles) -> Self {
        Self {
            format,
            document,
            game,
            dir,
            name,
            original,
            undo: Vec::new(),
            redo: Vec::new(),
            base: 0,
            extra: 0,
        }
    }

    /// The game of the format that opened the file.
    #[must_use]
    pub fn game(&self) -> Game {
        self.game
    }

    /// The folder the file was read from. A personal path: never log it raw.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// The file name inside [`Session::dir`]. A personal value: never log it
    /// raw.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The bytes of the last clean state: what `open` read, and later what a
    /// save wrote. [`Session::discard`] reopens them.
    #[must_use]
    pub fn original(&self) -> &SaveFiles {
        &self.original
    }

    /// The editing capabilities of the document.
    #[must_use]
    pub fn capabilities(&self) -> &[Capability] {
        self.document.capabilities()
    }

    /// The summary of the document as it stands now.
    ///
    /// # Errors
    ///
    /// The format's [`FormatError`] when the document cannot be read.
    pub fn summary(&mut self) -> Result<SaveSummary, FormatError> {
        self.document.summary()
    }

    /// The editable view of the document as it stands now.
    ///
    /// # Errors
    ///
    /// The format's [`FormatError`] when the document cannot be read.
    pub fn view(&mut self) -> Result<SaveView, FormatError> {
        self.document.view()
    }

    /// Whether the game's rules lock the row against the default mode.
    ///
    /// # Errors
    ///
    /// The format's [`EditError`] when the collection cannot be read.
    pub fn row_locked(&mut self, row: &RowId) -> Result<bool, EditError> {
        self.document.row_locked(row)
    }

    /// Whether the game's rules lock the field against the default mode.
    ///
    /// # Errors
    ///
    /// The format's [`EditError`] when the field cannot be read.
    pub fn field_locked(&self, field: &FieldId) -> Result<bool, EditError> {
        self.document.field_locked(field)
    }

    /// Applies `edit` in the default mode: the game's rules and caps hold.
    /// Success records the change and clears the redo stack.
    ///
    /// # Errors
    ///
    /// The document's [`EditError`]. The edit is all or none: on error the
    /// document and the history are unchanged.
    pub fn edit(&mut self, edit: &Edit) -> Result<(), EditError> {
        let change = self.document.apply_game(edit)?;
        self.record(change);
        Ok(())
    }

    /// Applies `edit` in expert mode: storage bounds only, no game cap and no
    /// game rule. Success records the change and clears the redo stack.
    ///
    /// # Errors
    ///
    /// The document's [`EditError`]. On error the document and the history
    /// are unchanged.
    pub fn edit_expert(&mut self, edit: &Edit) -> Result<(), EditError> {
        let inverse = self.document.apply(edit, CapPolicy::StorageOnly)?;
        self.record(GameChange::new(ChangeSet::from_iter([edit.clone()]), ChangeSet::from_iter([inverse])));
        Ok(())
    }

    /// Undoes the latest change. `Ok(false)` when there is nothing to undo.
    ///
    /// # Errors
    ///
    /// The document's [`EditError`], unchanged. The entry stays on the undo
    /// stack. After [`EditError::RollbackFailed`] the document state is
    /// unknown: call [`Session::discard`].
    pub fn undo(&mut self) -> Result<bool, EditError> {
        let Some(change) = self.undo.last() else { return Ok(false) };
        let _replayed = self.document.apply_all(&change.inverse, CapPolicy::StorageOnly)?;
        if let Some(change) = self.undo.pop() {
            self.redo.push(change);
        }
        Ok(true)
    }

    /// Redoes the latest undone change. `Ok(false)` when there is nothing to
    /// redo.
    ///
    /// # Errors
    ///
    /// The document's [`EditError`], unchanged. The entry stays on the redo
    /// stack. After [`EditError::RollbackFailed`] the document state is
    /// unknown: call [`Session::discard`].
    pub fn redo(&mut self) -> Result<bool, EditError> {
        let Some(change) = self.redo.last() else { return Ok(false) };
        let _replayed = self.document.apply_all(&change.forward, CapPolicy::StorageOnly)?;
        if let Some(change) = self.redo.pop() {
            self.undo.push(change);
        }
        Ok(true)
    }

    /// Whether [`Session::undo`] has a change to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether [`Session::redo`] has a change to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Whether the document differs from the clean point.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.pending_count() != 0
    }

    /// The number of changes between the clean point and the document.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.extra + self.undo.len().abs_diff(self.base)
    }

    /// Reopens the document from [`Session::original`], clears the history
    /// and resets the clean point. No file is read or written.
    ///
    /// # Errors
    ///
    /// [`Error::Format`] when the format refuses the bytes. The session is
    /// unchanged then.
    pub fn discard(&mut self) -> Result<(), Error> {
        let document = self.format.open(&self.original).map_err(|source| Error::Format { source })?;
        self.document = document;
        self.undo.clear();
        self.redo.clear();
        self.base = 0;
        self.extra = 0;
        Ok(())
    }

    fn record(&mut self, change: GameChange) {
        let depth = self.undo.len();
        if depth < self.base {
            self.extra += self.base - depth;
            self.base = depth;
        }
        self.undo.push(change);
        self.redo.clear();
    }
}
