//! The close veto over each modal dialog. A close request (the window X or
//! File > Quit) waits while another dialog covers the page, asks nothing and
//! changes nothing, and works again once that dialog closes. These tests call
//! `show()` on Slint's testing backend, which has no screen.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Machine, Rig, Site, closeable_rig, editing_site};
use i_slint_backend_testing::AccessibleRole::Button;
use neoffse_app::AboutDialog;
use slint::ComponentHandle;

/// A shown window with the first slot open and About shown.
fn opened(site: &Site, machine: Machine) -> (Rig, AboutDialog) {
    let (rig, about) = closeable_rig(site, machine);
    rig.ui.window.invoke_about_requested();
    assert!(about.window().is_visible());
    (rig, about)
}

/// Sends the X and File > Quit; each must leave the window, About, the
/// session and the dialogs as they are. `still_open` reads the modal under test.
fn assert_vetoed(rig: &Rig, about: &AboutDialog, still_open: impl Fn(&Rig) -> bool) {
    let ui = &rig.ui;
    let requests: [&dyn Fn(); 2] = [&|| ui.request_close(), &|| ui.window.invoke_quit_requested()];
    for request in requests {
        request();
        assert!(ui.window.window().is_visible());
        assert!(about.window().is_visible(), "About stays while the window stays");
        assert!(!ui.bridge().get_discard_dialog_open(), "the veto is silent");
        assert!(!ui.app.editor.leave_pending());
        assert!(ui.session_open());
        assert!(still_open(rig));
    }
}

/// Dirty session, then a close: the discard question opens.
fn assert_asks(rig: &Rig) {
    rig.ui.request_close();
    assert!(rig.ui.window.window().is_visible());
    assert!(rig.ui.bridge().get_discard_dialog_open());
    assert!(rig.ui.app.editor.leave_pending());
}

/// Clean session, then a close: the window hides.
fn assert_closes(rig: &Rig, about: &AboutDialog) {
    rig.ui.request_close();
    assert!(!rig.ui.window.window().is_visible());
    assert!(!about.window().is_visible());
}

#[test]
fn a_close_over_the_review_changes_nothing() {
    let site = editing_site();
    let (rig, about) = opened(&site, Machine::Quiet);
    let ui = &rig.ui;
    ui.make_dirty();
    ui.bridge().invoke_review_requested();
    assert!(ui.bridge().get_review_open());

    assert_vetoed(&rig, &about, |rig| rig.ui.bridge().get_review_open());
    assert_eq!(ui.bridge().get_pending_count(), 1);

    ui.element("Save", Button).invoke_accessible_default_action();
    ui.manual.run_all();

    assert_eq!(rig.gil_on_disk(), 500);
    assert!(ui.window.window().is_visible());
    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(!ui.bridge().get_review_open());
}

#[test]
fn a_close_after_the_review_is_cancelled_asks_again() {
    let site = editing_site();
    let (rig, about) = opened(&site, Machine::Quiet);
    let ui = &rig.ui;
    ui.make_dirty();
    ui.bridge().invoke_review_requested();
    ui.bridge().invoke_review_cancelled();
    assert!(!ui.bridge().get_review_open());

    assert_asks(&rig);
    assert!(about.window().is_visible());
}

#[test]
fn a_close_over_the_expert_warning_changes_nothing() {
    let site = editing_site();
    let (rig, about) = opened(&site, Machine::Quiet);
    let ui = &rig.ui;
    ui.make_dirty();
    ui.bridge().invoke_expert_toggle_requested();
    assert!(ui.bridge().get_expert_warning_open());

    assert_vetoed(&rig, &about, |rig| rig.ui.bridge().get_expert_warning_open());

    ui.bridge().invoke_expert_warning_cancelled();
    assert_asks(&rig);
}

#[test]
fn a_close_over_the_save_game_warning_changes_nothing() {
    let site = editing_site();
    let (rig, about) = opened(&site, Machine::GameRuns);
    let ui = &rig.ui;
    ui.make_dirty();
    ui.bridge().invoke_save_requested();
    ui.manual.run_all();
    assert!(ui.bridge().get_game_running_open());

    assert_vetoed(&rig, &about, |rig| rig.ui.bridge().get_game_running_open());

    ui.bridge().invoke_save_anyway_cancelled();
    assert_asks(&rig);
}

#[test]
fn a_close_over_the_changed_on_disk_dialog_changes_nothing() {
    let site = editing_site();
    let (rig, about) = opened(&site, Machine::Quiet);
    let ui = &rig.ui;
    ui.make_dirty();
    rig.change_on_disk(777);
    ui.bridge().invoke_save_requested();
    ui.manual.run_all();
    assert!(ui.bridge().get_changed_on_disk_open());

    assert_vetoed(&rig, &about, |rig| rig.ui.bridge().get_changed_on_disk_open());

    ui.bridge().invoke_reload_cancelled();
    assert_asks(&rig);
}

#[test]
fn a_close_over_the_restore_confirmation_changes_nothing() {
    let site = editing_site();
    let (rig, about) = opened(&site, Machine::Quiet);
    rig.backup_now("before-save");
    rig.open_page();
    rig.pick_backup();

    assert_vetoed(&rig, &about, |rig| rig.ui.restore_bridge().get_confirm_open());

    rig.ui.restore_bridge().invoke_restore_cancelled();
    assert_closes(&rig, &about);
}

#[test]
fn a_close_over_the_restore_game_warning_changes_nothing() {
    let site = editing_site();
    let (rig, about) = opened(&site, Machine::GameRuns);
    rig.backup_now("before-save");
    rig.open_page();
    rig.start_restore();
    rig.ui.manual.run_all();
    assert!(rig.ui.restore_bridge().get_game_running_open());

    assert_vetoed(&rig, &about, |rig| rig.ui.restore_bridge().get_game_running_open());

    rig.ui.restore_bridge().invoke_restore_anyway_cancelled();
    assert_closes(&rig, &about);
}
