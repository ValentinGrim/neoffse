//! Wires undo and redo, and writes the history counts the pending changes
//! bar reads.
//!
//! The callbacks hold a `slint::Weak<MainWindow>` and a clone of [`App`],
//! never a strong window handle. Undo and redo replay recorded values and
//! draw nothing, so they refresh directly and skip the "Stats raised" check
//! of the edit callbacks.

use slint::ComponentHandle;

use super::editor::refresh_with;
use crate::editor::History;
use crate::{App, EditNotice, EditorBridge, EditorViewModel, MainWindow};

/// Wires `undo-requested` and `redo-requested` of the [`EditorBridge`] to
/// `app.editor`.
///
/// Both do nothing while a save runs: the worker holds the session lock. A
/// refresh clears the notice, so an undo also removes the "Undo to cancel"
/// notice of a level increase.
pub fn connect_history(window: &MainWindow, app: &App) {
    let bridge = window.global::<EditorBridge>();
    bridge.on_undo_requested(replay(window, app, EditorViewModel::undo));
    bridge.on_redo_requested(replay(window, app, EditorViewModel::redo));
}

fn replay(window: &MainWindow, app: &App, step: fn(&EditorViewModel) -> Option<EditNotice>) -> impl Fn() + 'static {
    let (weak, app) = (window.as_weak(), app.clone());
    move || {
        if app.editor.is_saving() {
            return;
        }
        if let Some(window) = weak.upgrade() {
            let notice = step(&app.editor);
            refresh_with(&window, &app, notice);
        }
    }
}

/// Writes the pending count (saturating at `i32::MAX`) and the two step
/// flags of `history` into the bridge.
pub(crate) fn sync_history(bridge: &EditorBridge<'_>, history: History) {
    bridge.set_pending_count(i32::try_from(history.pending).unwrap_or(i32::MAX));
    bridge.set_can_undo(history.can_undo);
    bridge.set_can_redo(history.can_redo);
}
