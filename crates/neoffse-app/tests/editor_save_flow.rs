//! The save flow on the window: the Save button of the review, the working
//! state, the notices. Every save writes into temporary folders and runs
//! under a scripted process probe. The testing backend draws nothing, so a
//! test reads the bridge and the accessibility tree.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use common::{Machine, Rig, Ui, editing_site, rig};
use i_slint_backend_testing::AccessibleRole::{Button, Text};
use neoffse::io::dirs::DataDir;
use neoffse_app::editor::stamp_of;
use neoffse_app::{EditProblem, EditorCommand};
use slint::Model;
use slint::platform::Key;

fn quiet(site: &common::Site) -> Rig {
    rig(site, Machine::Quiet)
}

fn bar_shown(ui: &Ui) -> bool {
    ui.has_element("Review & save", Button)
}

/// Runs the queued save to its end.
fn finish(rig: &Rig) {
    rig.ui.manual.run_all();
}

#[test]
fn save_writes_backs_up_and_reports() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    let before = rig.bytes();
    ui.make_dirty();

    ui.bridge().invoke_save_requested();
    assert!(ui.bridge().get_working());
    assert!(ui.bridge().get_blocked());
    assert!(!ui.bridge().get_review_open());
    assert_eq!(rig.bytes(), before, "nothing is written before the worker runs");
    assert!(ui.has_element("Working...", Text), "the bar says so");

    finish(&rig);

    let notice = ui.bridge().get_notice();
    assert_eq!(notice.problem, EditProblem::Saved);
    assert_eq!(Some(notice.value.to_string()), stamp_of(&rig.newest_created()));
    assert!(ui.bridge().get_has_notice());
    assert_eq!(ui.bridge().get_pending_count(), 0);
    assert!(!bar_shown(ui));
    assert_ne!(rig.bytes(), before);
    assert_eq!(rig.gil_on_disk(), 500);
    assert_eq!(rig.backups(), 1);
    assert!(!ui.bridge().get_working());
    assert!(!ui.bridge().get_blocked());
}

#[test]
fn the_review_save_button_saves() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    ui.make_dirty();
    ui.chord(&[Key::Control], "s");
    assert!(ui.bridge().get_review_open());

    ui.element("Save", Button).invoke_accessible_default_action();
    assert!(ui.bridge().get_working());
    assert!(ui.bridge().get_blocked(), "blocked never drops between the review and the save");
    assert!(!ui.bridge().get_review_open());
    finish(&rig);

    assert_eq!(ui.bridge().get_notice().problem, EditProblem::Saved);
    assert_eq!(rig.gil_on_disk(), 500);
}

#[test]
fn the_saved_sentence_names_the_backup_time() {
    let site = editing_site();
    let rig = quiet(&site);
    rig.ui.make_dirty();
    rig.ui.bridge().invoke_save_requested();
    finish(&rig);

    let stamp = stamp_of(&rig.newest_created()).unwrap();
    assert!(rig.ui.has_element(&format!("Saved. Backup kept: {stamp}"), Text));
}

#[test]
fn edits_and_back_are_refused_while_working() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    ui.make_dirty();
    ui.bridge().invoke_save_requested();

    ui.commit(ui.gil().key, "600");
    assert_eq!(ui.bridge().get_notice().problem, EditProblem::NotEditable);
    assert_eq!(ui.pending(), 1);
    ui.window.invoke_back_requested();
    ui.window.invoke_home_requested();
    assert!(ui.window.get_editor_open());
    assert!(!ui.bridge().get_discard_dialog_open());
    ui.key(Key::Escape);
    assert!(!ui.bridge().get_discard_dialog_open());
    assert!(ui.window.get_editor_open());
    for label in ["Undo", "Redo", "Review & save"] {
        assert_eq!(ui.element(label, Button).accessible_enabled(), Some(false), "{label} while working");
    }
    ui.bridge().invoke_review_requested();
    assert!(!ui.bridge().get_review_open());

    finish(&rig);
    assert!(ui.window.get_editor_open());
    assert_eq!(rig.gil_on_disk(), 500);
}

#[test]
fn an_undo_after_the_save_brings_the_bar_back() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    ui.make_dirty();
    ui.bridge().invoke_save_requested();
    finish(&rig);

    ui.chord(&[Key::Control], "z");
    assert_eq!(ui.gil().text, "123456");
    assert_eq!(ui.pending(), 1);
    assert!(bar_shown(ui));
    assert_eq!(ui.element("Redo", Button).accessible_enabled(), Some(true));

    ui.chord(&[Key::Control], "y");
    assert_eq!(ui.gil().text, "500");
    assert_eq!(ui.pending(), 0);

    ui.commit(ui.gil().key, "600");
    ui.bridge().invoke_save_requested();
    finish(&rig);
    assert_eq!(rig.backups(), 2);
    assert_eq!(rig.gil_on_disk(), 600);
}

#[test]
fn a_failed_save_keeps_the_edit() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    let not_a_folder = rig.data.path().join("plain-file");
    fs::write(&not_a_folder, b"x").unwrap();
    ui.app.state.set_data_dir(DataDir::at(&not_a_folder));
    let before = rig.bytes();
    ui.make_dirty();

    ui.bridge().invoke_save_requested();
    finish(&rig);

    assert_eq!(ui.bridge().get_notice().problem, EditProblem::SaveFailed);
    assert_eq!(ui.pending(), 1);
    assert!(bar_shown(ui));
    assert_eq!(rig.bytes(), before);
    assert!(!ui.bridge().get_working());
    assert!(!ui.bridge().get_game_running_open());
    assert!(ui.has_element("Saving failed. Your changes are still open.", Text));
}

#[test]
fn no_data_dir_is_a_failed_save() {
    let site = editing_site();
    let ui = common::open_editor(&site);
    ui.make_dirty();

    ui.bridge().invoke_save_requested();

    assert_eq!(ui.manual.queued_jobs(), 0, "no job without a data folder");
    assert_eq!(ui.bridge().get_notice().problem, EditProblem::SaveFailed);
    assert!(!ui.bridge().get_working());
    assert_eq!(ui.pending(), 1);
}

#[test]
fn a_panicking_probe_is_a_failed_save() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    ui.app.editor.set_process_probe(|_| panic!("probe failure injected by the test"));
    ui.make_dirty();

    ui.bridge().invoke_save_requested();
    finish(&rig);

    assert_eq!(ui.bridge().get_notice().problem, EditProblem::SaveFailed);
    assert!(!ui.bridge().get_working());
    assert_eq!(ui.pending(), 1);
}

#[test]
fn the_saved_notice_and_the_bar_are_separate() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    ui.commit(ui.level().key, "34");
    assert_eq!(ui.bridge().get_notice().problem, EditProblem::StatsRaised);
    assert!(bar_shown(ui), "the level-up notice and the bar hold together");

    ui.bridge().invoke_save_requested();
    finish(&rig);

    assert_eq!(ui.bridge().get_notice().problem, EditProblem::Saved);
    assert!(!bar_shown(ui));
    ui.bridge().invoke_notice_dismissed();
    assert!(!ui.bridge().get_has_notice());
}

#[test]
fn a_save_does_not_change_expert_mode_or_the_page() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    ui.bridge().invoke_expert_warning_confirmed();
    ui.navigate(EditorCommand::Party, 0);
    ui.make_dirty();

    ui.bridge().invoke_save_requested();
    finish(&rig);

    assert!(ui.app.editor.expert());
    assert!(ui.bridge().get_expert_mode());
    assert_eq!(ui.bridge().get_command(), EditorCommand::Party);
    assert_eq!(ui.bridge().get_character(), 0);
    assert!(ui.window.get_editor_open());
}

#[test]
fn a_save_with_no_net_difference_still_saves() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    ui.commit(ui.gil().key, "500");
    ui.commit(ui.gil().key, "123456");
    ui.chord(&[Key::Control], "s");
    assert!(ui.bridge().get_review_open());
    assert_eq!(ui.bridge().get_review_rows().row_count(), 0);

    ui.element("Save", Button).invoke_accessible_default_action();
    finish(&rig);

    assert_eq!(ui.bridge().get_notice().problem, EditProblem::Saved);
    assert_eq!(rig.backups(), 1);
    assert_eq!(ui.bridge().get_pending_count(), 0);
}

/// The worker holds the session lock while the probe waits. An edit callback
/// on the UI thread must answer with the notice and not wait for that lock.
#[test]
fn an_edit_during_a_running_save_does_not_wait_for_it() {
    let site = editing_site();
    let rig = quiet(&site);
    let ui = &rig.ui;
    ui.make_dirty();
    let gil = ui.gil().key;
    let (entered_tx, entered) = mpsc::channel::<()>();
    let (release, release_rx) = mpsc::channel::<()>();
    let release_rx = Mutex::new(release_rx);
    let timed_out = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&timed_out);
    // The probe runs inside `Session::save`, under the session lock.
    ui.app.editor.set_process_probe(move |_| {
        entered_tx.send(()).unwrap();
        let waited = release_rx.lock().unwrap().recv_timeout(Duration::from_secs(10));
        flag.store(waited.is_err(), Ordering::SeqCst);
        Vec::new()
    });
    ui.bridge().invoke_save_requested();
    let manual = Arc::clone(&ui.manual);
    let worker = thread::spawn(move || manual.run_jobs());
    entered.recv_timeout(Duration::from_secs(10)).expect("the save reached the probe");

    ui.bridge().invoke_field_text_committed(gil, "600".into());

    assert_eq!(ui.bridge().get_notice().problem, EditProblem::NotEditable);
    assert!(ui.bridge().get_has_notice());
    assert_eq!(ui.bridge().get_pending_count(), 1);
    release.send(()).unwrap();
    worker.join().unwrap();
    assert!(!timed_out.load(Ordering::SeqCst), "the edit callback waited for the save");

    finish(&rig);
    assert_eq!(rig.gil_on_disk(), 500);
    assert_eq!(ui.bridge().get_notice().problem, EditProblem::Saved);
}
