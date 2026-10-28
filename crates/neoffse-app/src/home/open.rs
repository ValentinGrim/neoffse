//! The open flow: a click on a slot, or the file dialog, opens a `Session`
//! on a worker thread and stores it in the [`AppState`].
//!
//! The open reads the chosen file and nothing else: no write, no backup, no
//! temporary file.

use std::ffi::OsStr;
use std::fmt;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError};

use neoffse::core::Game;
use neoffse::{Error, Registry, Session};

use super::SlotTarget;
use super::rows::summary_row_of;
use crate::dispatch::{Dispatcher, WorkerPanic, run_async};
use crate::picker::FilePicker;
use crate::state::AppState;
use crate::{OpenProblem, SummaryRow};

/// How an open attempt ended.
///
/// The generated `OpenProblem` implements `PartialEq` but not `Eq`, so this
/// type does not either.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum OpenOutcome {
    /// The save is open in the [`AppState`].
    Opened,
    /// The user closed the file dialog. Nothing changed.
    Cancelled,
    /// The open failed. The state is as it was.
    Failed(OpenProblem),
}

/// What the worker brings back. The session is not in the state yet.
enum Worked {
    Opened(Box<Session>, SummaryRow),
    Cancelled,
    Failed(OpenProblem),
}

struct Shared {
    registry: Arc<Registry>,
    picker: Arc<dyn FilePicker>,
    state: Arc<AppState>,
    busy: AtomicBool,
}

/// Opens saves on worker threads and stores the result in the
/// [`AppState`].
///
/// Clones share one busy flag. The type is `Send + Sync`.
#[derive(Clone)]
pub struct OpenViewModel {
    shared: Arc<Shared>,
}

impl fmt::Debug for OpenViewModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenViewModel").field("busy", &self.is_busy()).finish_non_exhaustive()
    }
}

impl OpenViewModel {
    /// A view model that opens into `state`, asks `picker` for files and
    /// reads them through `registry`. Nothing is busy.
    #[must_use]
    pub fn new(registry: Arc<Registry>, picker: Arc<dyn FilePicker>, state: Arc<AppState>) -> Self {
        Self {
            shared: Arc::new(Shared {
                registry,
                picker,
                state,
                busy: AtomicBool::new(false),
            }),
        }
    }

    /// Whether an open or a dialog is running.
    #[must_use]
    pub fn is_busy(&self) -> bool {
        self.shared.busy.load(Ordering::SeqCst)
    }

    /// Opens the slot `target` names. `None` (a key the data lacks) gives
    /// `Failed(UnknownSlot)`, a game without a format `Failed(Unsupported)`,
    /// a running open `Failed(Busy)`. Those three call `done` at once and
    /// queue no job.
    ///
    /// `done` runs once on the UI thread. The busy flag is clear by then.
    pub fn open_slot(&self, target: Option<SlotTarget>, dispatcher: &Arc<dyn Dispatcher>, done: impl FnOnce(OpenOutcome) + Send + 'static) {
        let Some(target) = target else {
            done(OpenOutcome::Failed(OpenProblem::UnknownSlot));
            return;
        };
        if !target.supported {
            done(OpenOutcome::Failed(OpenProblem::Unsupported));
            return;
        }
        if !self.begin() {
            done(OpenOutcome::Failed(OpenProblem::Busy));
            return;
        }
        let worker = Arc::clone(&self.shared);
        self.run(dispatcher, done, move || {
            open_one(&worker.registry, &target.dir, &target.name, Some(target.game))
        });
    }

    /// Asks the picker for a file, then opens it with no game given: the
    /// registry tries each format, so a renamed file opens too. A cancelled
    /// dialog gives `Cancelled`; a running open gives `Failed(Busy)` at once
    /// and the picker is not asked.
    ///
    /// `done` runs once on the UI thread. The busy flag is clear by then.
    pub fn open_file(&self, dispatcher: &Arc<dyn Dispatcher>, done: impl FnOnce(OpenOutcome) + Send + 'static) {
        if !self.begin() {
            done(OpenOutcome::Failed(OpenProblem::Busy));
            return;
        }
        let worker = Arc::clone(&self.shared);
        self.run(dispatcher, done, move || match worker.picker.pick() {
            None => Worked::Cancelled,
            Some(path) => match (path.parent(), path.file_name().and_then(OsStr::to_str)) {
                (Some(dir), Some(name)) => open_one(&worker.registry, dir, name, None),
                _ => Worked::Failed(OpenProblem::Unreadable),
            },
        });
    }

    /// Opens the open save again from its file on disk, in place of the
    /// session. The unsaved changes of the old session are lost; the caller
    /// asked the user first. The game, folder and file name come from the
    /// old session.
    ///
    /// No save open gives `Failed(UnknownSlot)` and a running open
    /// `Failed(Busy)`, both at once with no job. A failed open keeps the old
    /// session. The expert flag is untouched.
    ///
    /// `done` runs once on the UI thread. The busy flag is clear by then.
    pub fn reopen(&self, dispatcher: &Arc<dyn Dispatcher>, done: impl FnOnce(OpenOutcome) + Send + 'static) {
        let Some(session) = self.shared.state.session() else {
            done(OpenOutcome::Failed(OpenProblem::UnknownSlot));
            return;
        };
        let (game, dir, name) = {
            let session = session.lock().unwrap_or_else(PoisonError::into_inner);
            (session.game(), session.dir().to_path_buf(), session.name().to_owned())
        };
        if !self.begin() {
            done(OpenOutcome::Failed(OpenProblem::Busy));
            return;
        }
        let worker = Arc::clone(&self.shared);
        self.run(dispatcher, done, move || open_one(&worker.registry, &dir, &name, Some(game)));
    }

    /// Takes the busy flag. `false` when it is already taken.
    fn begin(&self) -> bool {
        self.shared.busy.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok()
    }

    fn run(&self, dispatcher: &Arc<dyn Dispatcher>, done: impl FnOnce(OpenOutcome) + Send + 'static, work: impl FnOnce() -> Worked + Send + 'static) {
        let shared = Arc::clone(&self.shared);
        run_async(dispatcher, work, move |result| {
            let outcome = match result {
                Ok(Worked::Opened(session, summary)) => {
                    shared.state.set(*session, summary);
                    OpenOutcome::Opened
                }
                Ok(Worked::Cancelled) => OpenOutcome::Cancelled,
                Ok(Worked::Failed(problem)) => OpenOutcome::Failed(problem),
                Err(WorkerPanic { .. }) => OpenOutcome::Failed(OpenProblem::Internal),
            };
            shared.busy.store(false, Ordering::SeqCst);
            done(outcome);
        });
    }
}

/// Reads one file and its summary. A summary error drops the session.
fn open_one(registry: &Registry, dir: &Path, name: &str, game: Option<Game>) -> Worked {
    let mut session = match registry.open(dir, name, game) {
        Ok(session) => session,
        Err(error) => return Worked::Failed(problem_of(&error)),
    };
    match session.summary() {
        Ok(summary) => {
            let row = summary_row_of(&summary, name);
            Worked::Opened(Box::new(session), row)
        }
        // summary() reads no file: its only error is FormatError::Decode, which problem_of also shows as NotASave.
        Err(_) => Worked::Failed(OpenProblem::NotASave),
    }
}

/// The only place an error becomes a user-facing problem. The error's text
/// never leaves this function.
fn problem_of(error: &Error) -> OpenProblem {
    match error {
        Error::Format { .. } | Error::NoFormat { .. } => OpenProblem::NotASave,
        Error::UnsupportedGame { .. } => OpenProblem::Unsupported,
        _ => OpenProblem::Unreadable,
    }
}
