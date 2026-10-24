//! Wires the Restore page: the entry points (the Backups button of a Home
//! game group and File > Restore a backup...), Back, the notice's dismiss
//! button and the restore flow's dialogs (`docs/architecture.md#ui-architecture`).
//!
//! The callbacks hold a `slint::Weak<MainWindow>` and a clone of [`App`],
//! never a strong window handle. The page replaces Home or the editor while
//! it is open and touches neither: `editor-open`, the open session and the
//! editor navigation stay as they were, so Back shows what was below. No
//! value pushed here carries a path or an error's text.

use neoffse::Running;
use neoffse::core::Game;
use slint::{ComponentHandle, ModelRc, VecModel};

use super::restore_run::{begin_restore, confirm};
use crate::{App, EditorBridge, MainWindow, RestoreBridge, RestoreNotice};

/// Wires `backups-requested` and `restore-menu-requested` of the window and
/// the callbacks of the [`RestoreBridge`].
///
/// The Backups button does nothing while an open runs or a restore works. The
/// menu item does nothing without an open editor, while a dialog covers the
/// editor, or on the page itself. The Restore button of a row asks first; the
/// restore runs on a worker once the user confirms, and it asks again when the
/// game runs. Nothing on the page reacts while the restore works. Call it
/// after [`connect_save`](crate::connect_save).
pub fn connect_restore(window: &MainWindow, app: &App) {
    let bridge = window.global::<RestoreBridge>();
    let weak = window.as_weak();

    window.on_backups_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move |id| {
            let Some(window) = weak.upgrade() else { return };
            let Some(game) = Game::from_id(&id) else { return };
            if app.open.is_busy() || window.global::<RestoreBridge>().get_working() {
                return;
            }
            open_restore(&window, &app, game);
        }
    });
    window.on_restore_menu_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            let Some(window) = weak.upgrade() else { return };
            if !window.get_editor_open() || window.get_restore_open() || window.global::<EditorBridge>().get_blocked() {
                return;
            }
            if let Some(game) = app.state.game() {
                open_restore(&window, &app, game);
            }
        }
    });
    bridge.on_back_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            let Some(window) = weak.upgrade() else { return };
            let bridge = window.global::<RestoreBridge>();
            if bridge.get_working() {
                return;
            }
            window.set_restore_open(false);
            app.restore.close();
            bridge.set_has_notice(false);
            bridge.set_unreadable_count(0);
            bridge.set_rows(ModelRc::new(VecModel::default()));
        }
    });
    bridge.on_notice_dismissed({
        let weak = weak.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                window.global::<RestoreBridge>().set_has_notice(false);
            }
        }
    });
    connect_flow(window, app);
}

/// Wires the confirmation, the running-game warning and the restore itself.
fn connect_flow(window: &MainWindow, app: &App) {
    let bridge = window.global::<RestoreBridge>();
    let weak = window.as_weak();

    bridge.on_restore_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move |key| {
            let Some(window) = weak.upgrade() else { return };
            let bridge = window.global::<RestoreBridge>();
            if bridge.get_blocked() {
                return;
            }
            if let Some(row) = app.restore.select(key) {
                bridge.set_pending_stamp(row.stamp);
                bridge.set_confirm_open(true);
            } else if let Some(game) = app.restore.game() {
                // A stale key: the list changed under the page. List it again.
                let notice = app.restore.show(game);
                sync_restore(&window, &app, notice);
            }
        }
    });
    bridge.on_restore_cancelled({
        let weak = weak.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                window.global::<RestoreBridge>().set_confirm_open(false);
            }
        }
    });
    bridge.on_restore_confirmed({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            if let Some(window) = weak.upgrade() {
                confirm(&window, &app);
            }
        }
    });
    bridge.on_restore_anyway_confirmed({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            let Some(window) = weak.upgrade() else { return };
            if !window.global::<RestoreBridge>().get_working() {
                begin_restore(&window, &app, Running::Allow);
            }
        }
    });
    // The selection survives the warning: the user may start again.
    bridge.on_restore_anyway_cancelled(move || {
        if let Some(window) = weak.upgrade() {
            window.global::<RestoreBridge>().set_game_running_open(false);
        }
    });
}

/// Lists the backups of `game`, fills the page and shows it. Every dialog flag
/// and the working flag start cleared.
pub(crate) fn open_restore(window: &MainWindow, app: &App, game: Game) {
    let notice = app.restore.show(game);
    sync_restore(window, app, notice);
    let bridge = window.global::<RestoreBridge>();
    bridge.set_confirm_open(false);
    bridge.set_game_running_open(false);
    bridge.set_working(false);
    window.set_restore_open(true);
}

/// Pushes the listing into the bridge: the game title, a fresh row model, the
/// count of unreadable folders and `notice`. It reads no path.
pub(crate) fn sync_restore(window: &MainWindow, app: &App, notice: Option<RestoreNotice>) {
    let bridge = window.global::<RestoreBridge>();
    bridge.set_game_title(app.restore.title().into());
    bridge.set_rows(ModelRc::new(VecModel::from(app.restore.rows())));
    bridge.set_unreadable_count(i32::try_from(app.restore.unreadable_count()).unwrap_or(i32::MAX));
    bridge.set_has_notice(notice.is_some());
    if let Some(notice) = notice {
        bridge.set_notice(notice);
    }
}
