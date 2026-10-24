//! Runs a restore from the Restore page and shows how it ended
//! (`docs/architecture.md#save-safety`, restore bullets).
//!
//! The restore runs on a worker. The `working` flag of the [`RestoreBridge`]
//! blocks the page from the call until the outcome returns to the UI thread,
//! and the close request is vetoed meanwhile. A notice carries a date at most,
//! never a path or an error's text (`docs/architecture.md#errors`).

use neoffse::Running;
use slint::ComponentHandle;

use super::restore::sync_restore;
use crate::restore::RestoreOutcome;
use crate::{App, MainWindow, RestoreBridge, RestoreNotice, RestoreProblem};

/// Starts the restore of the selected backup under `running`.
///
/// `working` goes up before any dialog flag goes down, so `blocked` never
/// drops and the page does not take the focus back between the steps.
pub(crate) fn begin_restore(window: &MainWindow, app: &App, running: Running) {
    let bridge = window.global::<RestoreBridge>();
    bridge.set_working(true);
    bridge.set_confirm_open(false);
    bridge.set_game_running_open(false);
    bridge.set_has_notice(false);
    let (weak, done_app) = (window.as_weak(), app.clone());
    app.restore.restore(running, &app.dispatcher, move |outcome| {
        if let Some(window) = weak.upgrade() {
            end_restore(&window, &done_app, outcome);
        }
    });
}

/// Handles the outcome on the UI thread. The running-game dialog goes up
/// before `working` goes down, for the same reason as in [`begin_restore`].
fn end_restore(window: &MainWindow, app: &App, outcome: RestoreOutcome) {
    let bridge = window.global::<RestoreBridge>();
    match outcome {
        RestoreOutcome::Restored { safety, .. } => {
            // The open session is not touched yet: the editor learns about a
            // replaced file in the open-session step.
            let value = safety.unwrap_or_default();
            finish_restore(window, app, notice(RestoreProblem::Restored, &value));
        }
        RestoreOutcome::GameRunning => {
            bridge.set_game_running_open(true);
            bridge.set_working(false);
        }
        RestoreOutcome::FolderMissing => finish_without_change(window, app, RestoreProblem::FolderMissing),
        RestoreOutcome::Failed => finish_without_change(window, app, RestoreProblem::Failed),
        // The target may hold a mix of files, and the "Before restore" backup
        // may exist: the list refreshes so it shows, and Home scans again.
        RestoreOutcome::Incomplete => finish_restore(window, app, notice(RestoreProblem::Incomplete, "")),
        // Another restore is running and clears `working` itself.
        RestoreOutcome::Busy => {}
    }
}

/// Ends a restore that wrote nothing: the list stays as it was.
fn finish_without_change(window: &MainWindow, app: &App, problem: RestoreProblem) {
    window.global::<RestoreBridge>().set_working(false);
    sync_restore(window, app, Some(notice(problem, "")));
}

/// Ends a restore that may have written: `working` goes down, the page lists
/// the backups again (the new "Before restore" backup is the top row) and
/// shows `notice`. A list that cannot be read replaces the notice. Home scans
/// again, after a partial restore as after a full one, so a slot that was
/// unreadable shows its summary on return.
fn finish_restore(window: &MainWindow, app: &App, notice: RestoreNotice) {
    window.global::<RestoreBridge>().set_working(false);
    let listed = app.restore.game().and_then(|game| app.restore.show(game));
    sync_restore(window, app, Some(listed.unwrap_or(notice)));
    super::start_scan(&window.as_weak(), app);
}

fn notice(problem: RestoreProblem, value: &str) -> RestoreNotice {
    RestoreNotice { problem, value: value.into() }
}
