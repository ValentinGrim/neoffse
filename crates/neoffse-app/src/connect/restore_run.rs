//! Runs a restore from the Restore page and shows how it ended
//! (`docs/architecture.md#save-safety`, restore bullets).
//!
//! The restore runs on a worker. The `working` flag of the [`RestoreBridge`]
//! blocks the page from the call until the outcome returns to the UI thread,
//! and the close request is vetoed meanwhile. A notice carries a date at most,
//! never a path or an error's text (`docs/architecture.md#errors`).
//!
//! When the restore replaces the file the editor holds, the open session is
//! stale. A dirty session asks the discard question before the restore
//! starts, without closing; the session is replaced by a reopen once the
//! restore succeeds, and stays as it was when the restore does not.

use neoffse::Running;
use slint::ComponentHandle;

use super::editor::refresh_editor;
use super::guard::sync_expert;
use super::restore::sync_restore;
use crate::editor::Leave;
use crate::home::OpenOutcome;
use crate::restore::RestoreOutcome;
use crate::{App, EditorBridge, MainWindow, RestoreBridge, RestoreNotice, RestoreProblem};

/// The Restore button of the confirmation.
///
/// A restore that replaces the open file, with unsaved edits in the session,
/// asks the discard question first. The discard flag goes up before the
/// confirmation drops, so `blocked` never drops between the two. Any other
/// restore starts at once under [`Running::Refuse`].
pub(crate) fn confirm(window: &MainWindow, app: &App) {
    let bridge = window.global::<RestoreBridge>();
    if bridge.get_working() {
        return;
    }
    if app.restore.replaces_open_save() && app.editor.is_dirty() {
        app.editor.request_discard(Leave::Restore);
        window.global::<EditorBridge>().set_discard_dialog_open(true);
        bridge.set_confirm_open(false);
        return;
    }
    begin_restore(window, app, Running::Refuse);
}

/// Starts the restore the user confirmed through the discard dialog, or
/// through a save that ended the pending leave. The selection is still set.
pub(crate) fn resume(window: &MainWindow, app: &App) {
    begin_restore(window, app, Running::Refuse);
}

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
        RestoreOutcome::Restored { safety, replaced_open } => {
            let restored = notice(RestoreProblem::Restored, &safety.unwrap_or_default());
            if replaced_open {
                reopen(window, app, restored);
            } else {
                finish_restore(window, app, restored);
            }
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

/// Replaces the open session with the restored file. `working` stays up until
/// the reopen ends. On success the editor shows the restored data and the
/// navigation stays; on failure the old session was stale, so it closes and
/// the page says so (Back then lands on Home).
fn reopen(window: &MainWindow, app: &App, restored: RestoreNotice) {
    let (weak, done_app) = (window.as_weak(), app.clone());
    app.open.reopen(&app.dispatcher, move |outcome| {
        let Some(window) = weak.upgrade() else { return };
        if outcome == OpenOutcome::Opened {
            if let Some(summary) = done_app.editor.summary() {
                window.set_editor_summary(summary);
            }
            refresh_editor(&window, &done_app);
            finish_restore(&window, &done_app, restored);
        } else {
            done_app.editor.back();
            window.set_editor_open(false);
            done_app.home_size.restore(&window);
            sync_expert(&window, &done_app);
            finish_restore(&window, &done_app, notice(RestoreProblem::ReopenFailed, ""));
        }
    });
}

/// Ends a restore that wrote nothing: the list stays as it was.
fn finish_without_change(window: &MainWindow, app: &App, problem: RestoreProblem) {
    window.global::<RestoreBridge>().set_working(false);
    sync_restore(window, app, Some(notice(problem, "")));
}

/// Ends a restore that may have written: `working` goes down, the page lists
/// the backups again (the new "Before restore" backup is the top row) and
/// shows `notice`. A list that cannot be read replaces the notice, except
/// `Incomplete`: the user must learn the folder may hold a mix of files. The
/// page lists what it can, an empty list here. Home scans again, after a
/// partial restore as after a full one, so a slot that was unreadable shows
/// its summary on return.
fn finish_restore(window: &MainWindow, app: &App, notice: RestoreNotice) {
    window.global::<RestoreBridge>().set_working(false);
    let listed = app.restore.game().and_then(|game| app.restore.show(game));
    let shown = if notice.problem == RestoreProblem::Incomplete {
        notice
    } else {
        listed.unwrap_or(notice)
    };
    sync_restore(window, app, Some(shown));
    super::start_scan(&window.as_weak(), app);
}

fn notice(problem: RestoreProblem, value: &str) -> RestoreNotice {
    RestoreNotice { problem, value: value.into() }
}
