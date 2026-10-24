//! Wires the Home screen to its view models.
//!
//! Every callback holds a `slint::Weak<MainWindow>` and `Send` clones of the
//! view models, never a strong window handle: the window owns the callbacks,
//! so a strong handle would form a cycle. A task posted through the
//! dispatcher upgrades the weak handle and does nothing when the window is
//! gone.

pub mod editor;
pub mod guard;
pub mod history;
pub mod restore;
pub mod review;
pub mod save;

use std::fmt;
use std::sync::Arc;

use neoffse::Registry;
use neoffse::data::Language;
use neoffse::io::discovery::Environment;
use slint::{ComponentHandle, ModelRc, VecModel, Weak};

use self::editor::refresh_editor;
use self::guard::sync_expert;
use crate::dispatch::Dispatcher;
use crate::editor::{EditorViewModel, Leave, LeaveAnswer};
use crate::home::{HomeViewModel, OpenOutcome, OpenViewModel};
use crate::picker::FilePicker;
use crate::restore::RestoreViewModel;
use crate::state::AppState;
use crate::theme::sync_theme;
use crate::{EditorBridge, EditorCommand, MainWindow};

/// Everything the Home screen needs, built once in [`run`](crate::run).
///
/// The view models share one [`AppState`] that `new` creates.
#[derive(Clone)]
pub struct App {
    /// The Home list: scan, filter, slot rows.
    pub home: HomeViewModel,
    /// The open flow: a click or the file dialog.
    pub open: OpenViewModel,
    /// The editor page over the open save.
    pub editor: EditorViewModel,
    /// The Restore page over one game's backups.
    pub restore: RestoreViewModel,
    /// The open save, shared by `open`, `editor` and `restore`.
    pub state: Arc<AppState>,
    /// Where workers run and where their results return.
    pub dispatcher: Arc<dyn Dispatcher>,
}

impl fmt::Debug for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("App")
            .field("home", &self.home)
            .field("open", &self.open)
            .field("editor", &self.editor)
            .field("restore", &self.restore)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl App {
    /// Builds the view models over one empty [`AppState`]. Nothing runs yet.
    #[must_use]
    pub fn new(registry: Arc<Registry>, language: Language, environment: Environment, picker: Arc<dyn FilePicker>, dispatcher: Arc<dyn Dispatcher>) -> Self {
        let state = Arc::new(AppState::new());
        Self {
            home: HomeViewModel::new(Arc::clone(&registry), environment),
            open: OpenViewModel::new(Arc::clone(&registry), picker, Arc::clone(&state)),
            editor: EditorViewModel::new(Arc::clone(&state), language),
            restore: RestoreViewModel::new(registry, Arc::clone(&state)),
            state,
            dispatcher,
        }
    }
}

/// Wires the callbacks of `window` to `app` and starts the first scan.
///
/// Call it on the UI thread, before the event loop starts.
pub fn connect_home(window: &MainWindow, app: &App) {
    let weak = window.as_weak();

    window.on_owned_only_toggled({
        let (weak, app) = (weak.clone(), app.clone());
        move |owned_only| {
            app.home.set_owned_only(owned_only);
            with_window(&weak, &app, |_, _| {});
        }
    });
    window.on_rescan_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move || start_scan(&weak, &app)
    });
    window.on_slot_clicked({
        let (weak, app) = (weak.clone(), app.clone());
        move |key| {
            let done = open_done(&weak, &app);
            app.open.open_slot(app.home.slot_target(key), &app.dispatcher, done);
            with_window(&weak, &app, |_, _| {});
        }
    });
    window.on_open_file_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            let done = open_done(&weak, &app);
            app.open.open_file(&app.dispatcher, done);
            with_window(&weak, &app, |_, _| {});
        }
    });
    window.on_back_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            if let Some(window) = weak.upgrade()
                && !step_back(&window)
            {
                go_home(&window, &app);
            }
        }
    });
    window.on_home_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            if let Some(window) = weak.upgrade() {
                go_home(&window, &app);
            }
        }
    });
    window.on_problem_dismissed({
        let weak = weak.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                window.set_has_open_problem(false);
            }
        }
    });

    start_scan(&weak, app);
}

/// Leaves the editor for Home. Nothing happens while a save or a reload runs.
/// A dirty session asks first: the discard dialog opens and the editor stays
/// until the user decides.
fn go_home(window: &MainWindow, app: &App) {
    // A save or a reload runs: the worker may hold the session.
    if window.global::<EditorBridge>().get_working() {
        return;
    }
    if app.editor.leave(Leave::Back) == LeaveAnswer::Left {
        window.set_editor_open(false);
        sync_expert(window, app);
        refresh_home(window, app);
    } else {
        window.global::<EditorBridge>().set_discard_dialog_open(true);
    }
}

/// Steps back one level inside the editor: a character page or another
/// command returns to the Party cards. Returns `false` on the Party cards,
/// where Back does what Home does.
fn step_back(window: &MainWindow) -> bool {
    let bridge = window.global::<EditorBridge>();
    if bridge.get_command() == EditorCommand::Party && bridge.get_character() < 0 {
        return false;
    }
    reset_navigation(&bridge);
    true
}

/// Puts the editor on the Party cards.
fn reset_navigation(bridge: &EditorBridge<'_>) {
    bridge.set_command(EditorCommand::Party);
    bridge.set_character(-1);
}

/// Pushes the Home state into `window`: theme, games, scanning, busy, skipped
/// count, filter.
pub fn refresh_home(window: &MainWindow, app: &App) {
    sync_theme(window, app.state.game());
    window.set_games(ModelRc::new(VecModel::from(app.home.rows())));
    window.set_scanning(app.home.is_scanning());
    window.set_scan_failed(app.home.scan_failed());
    window.set_busy(app.open.is_busy());
    window.set_skipped_count(i32::try_from(app.home.skipped_count()).unwrap_or(i32::MAX));
    window.set_owned_only(app.home.owned_only());
}

/// Runs `action`, then [`refresh_home`], when the window is still alive.
fn with_window(weak: &Weak<MainWindow>, app: &App, action: impl FnOnce(&MainWindow, &App)) {
    if let Some(window) = weak.upgrade() {
        action(&window, app);
        refresh_home(&window, app);
    }
}

fn start_scan(weak: &Weak<MainWindow>, app: &App) {
    let (done_weak, done_app) = (weak.clone(), app.clone());
    app.home.start_scan(&app.dispatcher, move || with_window(&done_weak, &done_app, |_, _| {}));
    with_window(weak, app, |_, _| {});
}

/// The closure that ends an open on the UI thread.
fn open_done(weak: &Weak<MainWindow>, app: &App) -> impl FnOnce(OpenOutcome) + Send + 'static {
    let (weak, app) = (weak.clone(), app.clone());
    move |outcome| with_window(&weak, &app, |window, app| show_outcome(window, app, outcome))
}

fn show_outcome(window: &MainWindow, app: &App, outcome: OpenOutcome) {
    match outcome {
        OpenOutcome::Opened => {
            if let Some(summary) = app.editor.summary() {
                reset_navigation(&window.global::<EditorBridge>());
                refresh_editor(window, app);
                window.set_editor_summary(summary);
                window.set_editor_open(true);
                window.set_has_open_problem(false);
            }
        }
        OpenOutcome::Cancelled => {}
        OpenOutcome::Failed(problem) => {
            window.set_open_problem(problem);
            window.set_has_open_problem(true);
        }
    }
}
