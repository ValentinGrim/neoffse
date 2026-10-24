//! The editor page over an open session.

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use neoffse::Session;
use neoffse::core::{FieldId, RowId, TableKind};
use neoffse::data::Language;
use neoffse::io::process;

use super::layout::{FieldSlot, Layout, Locks, build};
use super::leave::Leave;
use super::notice::problem_notice;
use super::overview::overview;
use super::pick::picker_entries;
use crate::assets::{AssetProvider, default_provider};
use crate::state::AppState;
use crate::{EditNotice, EditProblem, EditorData, Overview, PickerEntry, SummaryRow};

/// Answers the builder's lock questions from a session. A failed query
/// counts as locked.
struct SessionLocks<'a>(&'a mut Session);

impl Locks for SessionLocks<'_> {
    fn field_locked(&mut self, field: &FieldId) -> bool {
        self.0.field_locked(field).unwrap_or(true)
    }

    fn row_locked(&mut self, row: &RowId) -> bool {
        self.0.row_locked(row).unwrap_or(true)
    }
}

/// The process scan a save runs under `Running::Refuse`: the executable names
/// of the game in, the names found running out.
pub(super) type Probe = Arc<dyn Fn(&[&'static str]) -> Vec<&'static str> + Send + Sync>;

pub(super) struct Shared {
    pub(super) state: Arc<AppState>,
    pub(super) language: Language,
    layout: Mutex<Layout>,
    pub(super) expert: AtomicBool,
    /// The leave request that waits for the user's answer.
    pub(super) pending: Mutex<Option<Leave>>,
    /// Set from the call of `save` to its `done`: the worker holds the
    /// session lock for that time.
    pub(super) saving: AtomicBool,
    pub(super) probe: Mutex<Probe>,
    /// Where the party cards ask for portraits.
    assets: Mutex<Arc<dyn AssetProvider>>,
}

/// The editor page: reads the open session, applies edits, keeps the expert
/// flag.
///
/// Clones share one state. The type is `Send + Sync`: the key table it keeps
/// holds plain Rust data, never a Slint model. `Debug` prints whether a save
/// is open and the expert flag.
#[derive(Clone)]
pub struct EditorViewModel {
    pub(super) shared: Arc<Shared>,
}

impl fmt::Debug for EditorViewModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EditorViewModel")
            .field("open", &self.shared.state.is_open())
            .field("expert", &self.expert())
            .finish()
    }
}

pub(super) fn not_editable() -> EditNotice {
    problem_notice(EditProblem::NotEditable, "")
}

pub(super) fn lock_session(session: &Mutex<Session>) -> MutexGuard<'_, Session> {
    session.lock().unwrap_or_else(PoisonError::into_inner)
}

impl EditorViewModel {
    /// An editor page over `state`. `language` picks the names of the items,
    /// spells and jobs the page shows.
    #[must_use]
    pub fn new(state: Arc<AppState>, language: Language) -> Self {
        Self {
            shared: Arc::new(Shared {
                state,
                language,
                layout: Mutex::new(Layout::default()),
                expert: AtomicBool::new(false),
                pending: Mutex::new(None),
                saving: AtomicBool::new(false),
                probe: Mutex::new(Arc::new(process::running)),
                assets: Mutex::new(default_provider()),
            }),
        }
    }

    pub(super) fn layout(&self) -> MutexGuard<'_, Layout> {
        self.shared.layout.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The summary of the open save, `None` when none is open.
    #[must_use]
    pub fn summary(&self) -> Option<SummaryRow> {
        self.shared.state.summary()
    }

    /// Leaves the editor now: closes the open save, forgets its keys and
    /// turns expert mode off. It asks nothing: the UI calls
    /// [`leave`](Self::leave), which asks first when the save holds unsaved
    /// edits.
    pub fn back(&self) {
        self.shared.state.close();
        *self.layout() = Layout::default();
        self.set_expert(false);
    }

    /// Rebuilds the data from the session and replaces the key table. Every
    /// key of an earlier call stops meaning anything. UI thread only: the
    /// result holds Slint models.
    ///
    /// # Errors
    ///
    /// An `internal` notice when no save is open or the view fails.
    pub fn data(&self) -> Result<EditorData, EditNotice> {
        let internal = || problem_notice(EditProblem::Internal, "");
        let built = {
            let session = self.shared.state.session().ok_or_else(internal)?;
            let mut session = lock_session(&session);
            let view = session.view().map_err(|_| internal());
            let game = session.game();
            view.map(|view| build(&view, game, self.shared.language, self.expert(), &mut SessionLocks(&mut session)))
        };
        match built {
            Ok(built) => {
                *self.layout() = built.layout;
                Ok(built.data)
            }
            Err(notice) => {
                *self.layout() = Layout::default();
                Err(notice)
            }
        }
    }

    /// The party overview of the open save. A default (`present` false) when
    /// no save is open or the view fails. It reads the session again, so the
    /// cards agree with [`data`](Self::data) after an edit. The result holds
    /// save content: never log it. UI thread only.
    #[must_use]
    pub fn overview(&self) -> Overview {
        let Some(session) = self.shared.state.session() else {
            return Overview::default();
        };
        // Clone the provider out of its lock: no lock is held while the
        // models build.
        let assets = Arc::clone(&self.shared.assets.lock().unwrap_or_else(PoisonError::into_inner));
        let mut session = lock_session(&session);
        let game = session.game();
        session
            .view()
            .map_or_else(|_| Overview::default(), |view| overview(&view, game, self.shared.language, &*assets))
    }

    /// Replaces the provider the party cards ask for portraits. The default
    /// is [`default_provider`]. Clones of the view model share the
    /// replacement; the caller refreshes the page to show it.
    pub fn set_assets(&self, assets: Arc<dyn AssetProvider>) {
        *self.shared.assets.lock().unwrap_or_else(PoisonError::into_inner) = assets;
    }

    /// The picker entries of field `key`'s tables that match `query`. Empty
    /// for a key the table lacks. Placeholders stay: the placeholder of an
    /// equipment slot is how a slot empties.
    #[must_use]
    pub fn field_entries(&self, key: i32, query: &str) -> Vec<PickerEntry> {
        let kinds = self.field_slot(key).map(|slot| slot.meta.tables).unwrap_or_default();
        self.entries(&kinds, query, false)
    }

    /// The picker entries of list `list`'s key tables that match `query`.
    /// Empty for a list the table lacks. Placeholders drop out: the game
    /// interface refuses to add a placeholder row
    /// (`docs/formats/pixel-remaster.md#inventory`).
    #[must_use]
    pub fn list_entries(&self, list: i32, query: &str) -> Vec<PickerEntry> {
        let kinds = usize::try_from(list)
            .ok()
            .and_then(|index| self.layout().lists.get(index).map(|meta| meta.key.tables.clone()))
            .unwrap_or_default();
        self.entries(&kinds, query, true)
    }

    /// Whether edits go through the raw interface. `false` at start.
    #[must_use]
    pub fn expert(&self) -> bool {
        self.shared.expert.load(Ordering::Relaxed)
    }

    /// Turns expert mode on or off. The caller refreshes with `data`.
    pub fn set_expert(&self, on: bool) {
        self.shared.expert.store(on, Ordering::Relaxed);
    }

    /// Whether the open save holds unsaved edits. `false` when none is open.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.shared.state.session().is_some_and(|session| lock_session(&session).is_dirty())
    }

    pub(super) fn field_slot(&self, key: i32) -> Option<FieldSlot> {
        usize::try_from(key).ok().and_then(|index| self.layout().fields.get(index).cloned())
    }

    fn entries(&self, kinds: &[TableKind], query: &str, skip_placeholders: bool) -> Vec<PickerEntry> {
        let tables = self.layout().tables;
        tables.map_or_else(Vec::new, |tables| picker_entries(tables, kinds, self.shared.language, query, skip_placeholders))
    }
}
