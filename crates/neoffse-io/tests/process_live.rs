//! `snapshot` and `running` against the live process table, read only. No
//! test needs a game, and none prints a process name, a command line or a path.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::sync::mpsc;
use std::thread;

use neoffse_io::process::{Key, matching, running, snapshot};

/// No executable matched.
const NONE: [&str; 0] = [];

#[test]
fn a_nonsense_executable_is_not_running() {
    assert_eq!(running(&["neoffse-no-such-game-7c1e.exe"]), NONE);
    assert_eq!(running(&[]), NONE);
}

#[test]
fn the_scan_sees_this_test_process() {
    let Some(own_name) = std::env::current_exe().ok().and_then(|path| path.file_name().map(ToOwned::to_owned)) else {
        return;
    };
    let Some(own_name) = own_name.to_str() else {
        return;
    };
    let key = Key::platform();
    assert_eq!(matching(&snapshot(key), key, &[own_name]), [own_name]);
    assert_eq!(running(&[own_name]), [own_name]);
}

#[test]
fn the_name_key_reads_no_command_line() {
    assert!(snapshot(Key::Name).iter().all(|process| process.argv0.is_none()));
    assert!(snapshot(Key::CommandLine).iter().any(|process| process.argv0.is_some()));
}

#[test]
fn threads_are_not_listed_as_processes() {
    const THREAD_NAME: &str = "neoffse-t-9c4e";
    let (release, hold) = mpsc::channel::<()>();
    let (ready, started) = mpsc::channel::<()>();
    let handle = thread::Builder::new()
        .name(THREAD_NAME.to_owned())
        .spawn(move || {
            ready.send(()).ok();
            hold.recv().ok();
        })
        .unwrap();
    started.recv().unwrap();
    let listed = snapshot(Key::CommandLine).iter().any(|process| process.name == THREAD_NAME);
    release.send(()).unwrap();
    handle.join().unwrap();
    assert!(!listed);
}
