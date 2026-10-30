//! `EventLoopDispatcher::post` on the testing backend's event loop. The
//! backend starts once per process, so this file holds one test. No window
//! is created or shown.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};
use std::time::Duration;

use neoffse_app::{Dispatcher, EventLoopDispatcher};

#[test]
fn post_runs_the_task_once_on_the_event_loop_thread() {
    i_slint_backend_testing::init_integration_test_with_system_time();
    let ui_thread = thread::current().id();

    let ran: Arc<Mutex<Vec<ThreadId>>> = Arc::default();
    let recorder = Arc::clone(&ran);
    let poster = thread::spawn(move || {
        EventLoopDispatcher.post(Box::new(move || {
            recorder.lock().unwrap_or_else(PoisonError::into_inner).push(thread::current().id());
            slint::quit_event_loop().unwrap();
        }));
    });

    // The guard ends a stuck loop after 5 s and flags the failure.
    let timed_out = Arc::new(AtomicBool::new(false));
    let (finished, waiting) = mpsc::channel::<()>();
    let flag = Arc::clone(&timed_out);
    let guard = thread::spawn(move || {
        if waiting.recv_timeout(Duration::from_secs(5)) == Err(mpsc::RecvTimeoutError::Timeout) {
            flag.store(true, Ordering::SeqCst);
            let _ = slint::quit_event_loop();
        }
    });

    slint::run_event_loop_until_quit().unwrap();
    drop(finished);
    guard.join().unwrap();
    poster.join().unwrap();

    assert!(!timed_out.load(Ordering::SeqCst), "the posted task never ran");
    assert_eq!(*ran.lock().unwrap_or_else(PoisonError::into_inner), vec![ui_thread]);
}
