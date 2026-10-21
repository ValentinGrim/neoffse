//! Helpers for the open flow: a recording picker, an outcome log, the
//! view models over one synthetic machine.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};

use neoffse_app::FilePicker;
use neoffse_app::home::{HomeViewModel, OpenOutcome, OpenViewModel, SlotTarget};
use neoffse_app::state::{AppState, EditorViewModel};

use super::{Manual, Site, registry, scanned};

/// A picker that returns a fixed answer and records who asked.
pub(crate) struct FixedPicker {
    answer: Option<PathBuf>,
    calls: Mutex<Vec<ThreadId>>,
}

impl FixedPicker {
    pub(crate) fn new(answer: Option<PathBuf>) -> Arc<Self> {
        Arc::new(Self {
            answer,
            calls: Mutex::new(Vec::new()),
        })
    }

    /// The ids of the threads that called `pick`, in call order.
    pub(crate) fn calls(&self) -> Vec<ThreadId> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }
}

impl FilePicker for FixedPicker {
    fn pick(&self) -> Option<PathBuf> {
        self.calls.lock().unwrap_or_else(PoisonError::into_inner).push(thread::current().id());
        self.answer.clone()
    }
}

/// Where `done` ran, and with what.
#[derive(Clone, Default)]
pub(crate) struct Outcomes {
    log: Arc<Mutex<Vec<(OpenOutcome, ThreadId)>>>,
}

impl Outcomes {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// A `done` closure that appends to this log.
    pub(crate) fn recorder(&self) -> impl FnOnce(OpenOutcome) + Send + 'static {
        let log = Arc::clone(&self.log);
        move |outcome| log.lock().unwrap_or_else(PoisonError::into_inner).push((outcome, thread::current().id()))
    }

    pub(crate) fn all(&self) -> Vec<(OpenOutcome, ThreadId)> {
        self.log.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// The outcomes alone, in order.
    pub(crate) fn outcomes(&self) -> Vec<OpenOutcome> {
        self.all().into_iter().map(|(outcome, _)| outcome).collect()
    }
}

/// A scanned Home, the open view model and the editor over one state.
pub(crate) struct Flow {
    pub(crate) home: HomeViewModel,
    pub(crate) open: OpenViewModel,
    pub(crate) editor: EditorViewModel,
    pub(crate) state: Arc<AppState>,
    pub(crate) picker: Arc<FixedPicker>,
    pub(crate) manual: Arc<Manual>,
}

impl Flow {
    /// Scans `site` to the end, then builds the open flow with a picker that
    /// answers `pick`.
    pub(crate) fn new(site: &Site, pick: Option<PathBuf>) -> Self {
        let home = scanned(site);
        let state = Arc::new(AppState::new());
        let picker = FixedPicker::new(pick);
        let open = OpenViewModel::new(registry(), Arc::clone(&picker) as Arc<dyn FilePicker>, Arc::clone(&state));
        let editor = EditorViewModel::new(Arc::clone(&state));
        Self {
            home,
            open,
            editor,
            state,
            picker,
            manual: Manual::new(),
        }
    }

    /// The target of the slot whose file is `name`, through the Home data.
    pub(crate) fn target(&self, name: &str) -> Option<SlotTarget> {
        (0..64).find_map(|key| self.home.slot_target(key).filter(|target| target.name == name))
    }

    /// Clicks the slot of file `name` and records the outcome in `outcomes`.
    pub(crate) fn click(&self, name: &str, outcomes: &Outcomes) {
        self.open.open_slot(self.target(name), &self.manual.dispatcher(), outcomes.recorder());
    }
}
