//! Wires the review dialog: the list of net changes the user checks before
//! a save (`docs/architecture.md#save-safety`, step 2).
//!
//! The callbacks hold a `slint::Weak<MainWindow>` and a clone of [`App`],
//! never a strong window handle. Opening the review reads the session on the
//! UI thread, as the edit callbacks do; nothing here writes a file.

use slint::{ComponentHandle, ModelRc, VecModel};

use super::editor::refresh_with;
use crate::{App, EditorBridge, MainWindow};

/// Wires `review-requested` and `review-cancelled` of the [`EditorBridge`].
///
/// `review-requested` (the bar's button and Ctrl+S) opens nothing while a
/// save runs, while a dialog covers the page or while the session is clean.
/// `review-cancelled` closes the dialog and forgets a pending leave: the
/// user stays in the editor.
pub fn connect_review(window: &MainWindow, app: &App) {
    let bridge = window.global::<EditorBridge>();
    let weak = window.as_weak();

    bridge.on_review_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            let Some(window) = weak.upgrade() else { return };
            // `history` locks the session, which a running save holds.
            if app.editor.is_saving() || window.global::<EditorBridge>().get_blocked() || app.editor.history().pending == 0 {
                return;
            }
            open_review(&window, &app);
        }
    });
    bridge.on_review_cancelled({
        let app = app.clone();
        move || {
            let Some(window) = weak.upgrade() else { return };
            window.global::<EditorBridge>().set_review_open(false);
            if app.editor.leave_pending() {
                app.editor.cancel_leave();
            }
        }
    });
}

/// Rebuilds the review rows from the session and opens the dialog. Returns
/// `false` when nothing opened: the view model answered with a notice, which
/// the page then shows.
pub(crate) fn open_review(window: &MainWindow, app: &App) -> bool {
    match app.editor.review() {
        Ok(rows) => {
            let bridge = window.global::<EditorBridge>();
            bridge.set_review_rows(ModelRc::new(VecModel::from(rows)));
            bridge.set_review_open(true);
            true
        }
        Err(notice) => {
            refresh_with(window, app, Some(notice));
            false
        }
    }
}
