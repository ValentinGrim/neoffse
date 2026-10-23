//! Wires the save flow: the Save button of the review, the running-game
//! warning, the changed-on-disk dialog and the notices that end a save
//! (`docs/architecture.md#save-safety`).
//!
//! The callbacks hold a `slint::Weak<MainWindow>` and a clone of [`App`],
//! never a strong window handle. The save and the reload run on a worker; the
//! `working` flag of the [`EditorBridge`] blocks the page until the outcome
//! returns to the UI thread. A notice never carries an error's text.

use neoffse::Running;
use slint::ComponentHandle;

use super::editor::{refresh_editor, refresh_with};
use crate::editor::{SaveOutcome, reload_failed_notice, save_failed_notice, saved_notice};
use crate::home::OpenOutcome;
use crate::{App, EditorBridge, MainWindow};

/// Wires `save-requested`, `save-anyway-confirmed`, `save-anyway-cancelled`,
/// `reload-confirmed` and `reload-cancelled` of the [`EditorBridge`].
///
/// A save that meets a running game opens the warning, and one that meets a
/// file changed on disk opens the reload dialog. Both dialogs leave the
/// edits in place until the user answers. Call it after
/// [`connect_review`](crate::connect_review).
pub fn connect_save(window: &MainWindow, app: &App) {
    let bridge = window.global::<EditorBridge>();
    let weak = window.as_weak();

    bridge.on_save_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            if app.editor.is_saving() {
                return;
            }
            if let Some(window) = weak.upgrade() {
                begin_save(&window, &app, Running::Refuse);
            }
        }
    });
    bridge.on_save_anyway_confirmed({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            if app.editor.is_saving() {
                return;
            }
            if let Some(window) = weak.upgrade() {
                begin_save(&window, &app, Running::Allow);
            }
        }
    });
    bridge.on_save_anyway_cancelled({
        let weak = weak.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                window.global::<EditorBridge>().set_game_running_open(false);
            }
        }
    });
    bridge.on_reload_confirmed({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            if let Some(window) = weak.upgrade() {
                begin_reload(&window, &app);
            }
        }
    });
    bridge.on_reload_cancelled(move || {
        if let Some(window) = weak.upgrade() {
            window.global::<EditorBridge>().set_changed_on_disk_open(false);
        }
    });
}

/// Starts a save under `running`. `working` goes up before any dialog flag
/// goes down, so `blocked` never drops and the page does not take the focus
/// back between the steps.
fn begin_save(window: &MainWindow, app: &App, running: Running) {
    let bridge = window.global::<EditorBridge>();
    bridge.set_working(true);
    bridge.set_review_open(false);
    bridge.set_game_running_open(false);
    let (weak, done_app) = (window.as_weak(), app.clone());
    app.editor.save(running, &app.dispatcher, move |outcome| {
        if let Some(window) = weak.upgrade() {
            end_save(&window, &done_app, outcome);
        }
    });
}

/// Handles the outcome on the UI thread. A dialog flag goes up before
/// `working` goes down, for the same reason as in [`begin_save`].
fn end_save(window: &MainWindow, app: &App, outcome: SaveOutcome) {
    let bridge = window.global::<EditorBridge>();
    match outcome {
        SaveOutcome::Saved { stamp } => {
            bridge.set_working(false);
            refresh_with(window, app, Some(saved_notice(stamp.as_deref())));
        }
        SaveOutcome::GameRunning => {
            bridge.set_game_running_open(true);
            bridge.set_working(false);
        }
        SaveOutcome::ChangedOnDisk => {
            bridge.set_changed_on_disk_open(true);
            bridge.set_working(false);
        }
        SaveOutcome::Failed => {
            bridge.set_working(false);
            refresh_with(window, app, Some(save_failed_notice()));
        }
        // Another save is running and clears `working` itself.
        SaveOutcome::Busy => {}
    }
}

/// Opens the save again from disk, in the same place of the editor. A failed
/// reload keeps the old session and says so.
fn begin_reload(window: &MainWindow, app: &App) {
    let bridge = window.global::<EditorBridge>();
    bridge.set_working(true);
    bridge.set_changed_on_disk_open(false);
    let (weak, done_app) = (window.as_weak(), app.clone());
    app.open.reopen(&app.dispatcher, move |outcome| {
        let Some(window) = weak.upgrade() else { return };
        let bridge = window.global::<EditorBridge>();
        if outcome == OpenOutcome::Opened {
            if let Some(summary) = done_app.editor.summary() {
                window.set_editor_summary(summary);
            }
            refresh_editor(&window, &done_app);
            bridge.set_working(false);
        } else {
            bridge.set_working(false);
            refresh_with(&window, &done_app, Some(reload_failed_notice()));
        }
    });
}
