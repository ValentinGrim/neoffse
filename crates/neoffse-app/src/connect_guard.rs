//! Wires expert mode, the discard dialog and the close request.
//!
//! The callbacks hold a `slint::Weak<MainWindow>` and a clone of [`App`],
//! never a strong window handle. The view model owns the state (the expert
//! flag, the pending leave request); the bridge flags only mirror it, and
//! `sync_expert` rewrites them after every change.

use slint::{CloseRequestResponse, ComponentHandle, Weak};

use crate::connect::refresh_home;
use crate::connect_editor::refresh_editor;
use crate::editor::{Leave, LeaveAnswer};
use crate::{AboutDialog, App, EditorBridge, MainWindow};

/// Wires expert mode and the discard dialog.
///
/// Expert mode starts off and resets when a save closes. Turning it on goes
/// through a warning; turning it off does not. The discard dialog answers the
/// leave request the Back button or [`connect_close`] raised.
pub fn connect_expert(window: &MainWindow, app: &App) {
    let bridge = window.global::<EditorBridge>();
    let weak = window.as_weak();
    sync_expert(window, app);

    bridge.on_expert_toggle_requested({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            let Some(window) = weak.upgrade() else { return };
            if app.editor.expert() {
                app.editor.set_expert(false);
                sync_expert(&window, &app);
                refresh_editor(&window, &app);
            } else {
                window.global::<EditorBridge>().set_expert_warning_open(true);
                // Slint toggled the menu checkmark before this call: the
                // mode stays off until the user confirms.
                sync_expert(&window, &app);
            }
        }
    });
    bridge.on_expert_warning_confirmed({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            let Some(window) = weak.upgrade() else { return };
            app.editor.set_expert(true);
            window.global::<EditorBridge>().set_expert_warning_open(false);
            sync_expert(&window, &app);
            refresh_editor(&window, &app);
        }
    });
    bridge.on_expert_warning_cancelled({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            let Some(window) = weak.upgrade() else { return };
            window.global::<EditorBridge>().set_expert_warning_open(false);
            // The mode did not change: the notice that led here stays.
            sync_expert(&window, &app);
        }
    });
    bridge.on_discard_confirmed({
        let (weak, app) = (weak.clone(), app.clone());
        move || {
            let Some(window) = weak.upgrade() else { return };
            let why = app.editor.confirm_discard();
            window.global::<EditorBridge>().set_discard_dialog_open(false);
            sync_expert(&window, &app);
            match why {
                Some(Leave::Back) => {
                    window.set_editor_open(false);
                    refresh_home(&window, &app);
                }
                // The session is gone, so the close handler takes the clean path.
                Some(_) => window.window().dispatch_event(slint::platform::WindowEvent::CloseRequested),
                None => {}
            }
        }
    });
    bridge.on_discard_cancelled({
        let (weak, app) = (weak, app.clone());
        move || {
            app.editor.cancel_leave();
            if let Some(window) = weak.upgrade() {
                window.global::<EditorBridge>().set_discard_dialog_open(false);
            }
        }
    });
}

/// Replaces the main window's close handler: a dirty session asks first.
///
/// Call it after [`connect_about`](crate::connect_about):
/// `Window::on_close_requested` keeps one handler, and this one hides the
/// About dialog itself when the close goes through. A clean session or none
/// at all closes at once. A dirty one keeps the window shown and opens the
/// discard dialog.
pub fn connect_close(window: &MainWindow, about: &AboutDialog, app: &App) {
    let (weak, about, app) = (window.as_weak(), about.as_weak(), app.clone());
    window.window().on_close_requested(move || {
        let left = app.editor.leave(Leave::Close) == LeaveAnswer::Left;
        let window = weak.upgrade();
        if left {
            hide_about(&about);
            if let Some(window) = window {
                sync_expert(&window, &app);
            }
            CloseRequestResponse::HideWindow
        } else {
            if let Some(window) = window {
                window.global::<EditorBridge>().set_discard_dialog_open(true);
            }
            CloseRequestResponse::KeepWindowShown
        }
    });
}

/// Without this hide, an open About dialog keeps the event loop, and the
/// process, alive after the main window closes.
fn hide_about(about: &Weak<AboutDialog>) {
    if let Some(dialog) = about.upgrade() {
        // The app has no logger yet; M4 routes show and hide errors through `tracing`.
        let _ = dialog.hide();
    }
}

/// Writes the expert flags of the bridge from the view model: the mode, the
/// menu checkmark and the offer on the notice.
///
/// The offer stays only while the shown notice is an above-cap refusal and
/// the mode is off.
pub(crate) fn sync_expert(window: &MainWindow, app: &App) {
    let bridge = window.global::<EditorBridge>();
    let expert = app.editor.expert();
    bridge.set_expert_mode(expert);
    bridge.set_expert_menu_checked(expert);
    bridge.set_offer_expert(bridge.get_has_notice() && app.editor.offers_expert(&bridge.get_notice()));
}
