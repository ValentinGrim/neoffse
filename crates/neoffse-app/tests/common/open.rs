//! Helpers for the open flow: a recording picker, an outcome log, the
//! view models over one synthetic machine.

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};

use neoffse::data::Language;
use neoffse::pr::slots::SLOT_FILES;
use neoffse_app::EditorViewModel;
use neoffse_app::FilePicker;
use neoffse_app::home::{HomeViewModel, OpenOutcome, OpenViewModel, SlotTarget};
use neoffse_app::state::AppState;

use neoffse::Registry;

use super::{ID_1, Manual, Site, registry, scanned_with, steam_folder};

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
        Self::with_registry(site, pick, registry())
    }

    /// [`Flow::new`] over `registry`: the scan and the open flow share it.
    pub(crate) fn with_registry(site: &Site, pick: Option<PathBuf>, registry: Arc<Registry>) -> Self {
        let home = scanned_with(site, Arc::clone(&registry));
        let state = Arc::new(AppState::new());
        let picker = FixedPicker::new(pick);
        let open = OpenViewModel::new(registry, Arc::clone(&picker) as Arc<dyn FilePicker>, Arc::clone(&state));
        let editor = EditorViewModel::new(Arc::clone(&state), Language::En);
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

/// A private FF I slot file copied into the FF I Steam folder of a synthetic
/// machine. The site deletes the copy on drop.
pub(crate) struct PrivateSlot {
    pub(crate) name: String,
    pub(crate) site: Site,
    pub(crate) copy: PathBuf,
}

/// One [`PrivateSlot`] per FF I slot file of the private saves. Returns
/// `None` after reporting why when the saves hold no slot file, so the test
/// returns early and says so.
pub(crate) fn private_slot_copies() -> Option<Vec<PrivateSlot>> {
    let paths = neoffse_testkit::private_saves("ff1")?;
    let slots: Vec<PrivateSlot> = paths
        .iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?;
            SLOT_FILES.iter().any(|file| file.name == name).then(|| {
                let site = Site::new();
                let copy = steam_folder(&site, 0, ID_1).join(name);
                fs::copy(path, &copy).unwrap();
                PrivateSlot {
                    name: name.to_owned(),
                    site,
                    copy,
                }
            })
        })
        .collect();
    if slots.is_empty() {
        neoffse_testkit::skip("no FF I slot file in the private saves");
        return None;
    }
    Some(slots)
}
