//! Tests of the plumbing: locale mapping, the picker trait and the
//! dispatcher. None of them opens a dialog, reads the system locale or needs
//! an event loop.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use neoffse::data::Language;
use neoffse_app::dispatch::{Dispatcher, EventLoopDispatcher, WorkerPanic, run_async};
use neoffse_app::language::language_for_locale;
use neoffse_app::picker::{FilePicker, NativePicker};

mod common;

use common::Manual;

#[test]
fn locale_with_primary_fr_gives_french() {
    for tag in ["fr", "FR", "fr-FR", "fr_CA.UTF-8", "fr@euro"] {
        assert_eq!(language_for_locale(Some(tag)), Language::Fr, "tag {tag}");
    }
}

#[test]
fn other_locales_give_english() {
    for tag in ["en-US", "de", "C", "POSIX", "", "fra", "fr2", "french", "frisian"] {
        assert_eq!(language_for_locale(Some(tag)), Language::En, "tag {tag:?}");
    }
    assert_eq!(language_for_locale(None), Language::En);
}

struct Fixed(Option<PathBuf>);

impl FilePicker for Fixed {
    fn pick(&self) -> Option<PathBuf> {
        self.0.clone()
    }
}

#[test]
fn picker_is_object_safe_and_fakeable() {
    let some: Arc<dyn FilePicker> = Arc::new(Fixed(Some(PathBuf::from("chosen"))));
    let none: Arc<dyn FilePicker> = Arc::new(Fixed(None));
    assert_eq!(some.pick(), Some(PathBuf::from("chosen")));
    assert_eq!(none.pick(), None);
    // Coerced, never called: `pick` opens a dialog.
    let _native: Arc<dyn FilePicker> = Arc::new(NativePicker);
}

#[test]
fn spawn_runs_the_job_on_another_thread() {
    let (sender, receiver) = mpsc::channel();
    EventLoopDispatcher.spawn(Box::new(move || {
        sender.send(thread::current().id()).unwrap();
    }));
    let worker = receiver.recv_timeout(Duration::from_secs(10)).expect("the job ran");
    assert_ne!(worker, thread::current().id());
}

#[test]
fn run_async_works_off_thread_then_posts_the_result() {
    let manual = Manual::new();
    let dispatcher = manual.dispatcher();
    let worked = Arc::new(Mutex::new(None));
    let delivered = Arc::new(Mutex::new(None));

    let worked_in = Arc::clone(&worked);
    let delivered_in = Arc::clone(&delivered);
    run_async(
        &dispatcher,
        move || {
            *worked_in.lock().unwrap() = Some(thread::current().id());
            42_u32
        },
        move |result| {
            *delivered_in.lock().unwrap() = Some((result, thread::current().id()));
        },
    );

    assert_eq!(manual.queued_jobs(), 1);
    assert!(worked.lock().unwrap().is_none(), "work ran before the job was run");
    assert!(delivered.lock().unwrap().is_none());

    let job_threads = manual.run_jobs();
    assert_eq!(job_threads.len(), 1);
    assert_ne!(job_threads[0], thread::current().id());
    assert_ne!(*worked.lock().unwrap(), Some(thread::current().id()));
    assert_eq!(manual.queued_posts(), 1);
    assert!(delivered.lock().unwrap().is_none(), "then ran before the post was run");

    manual.run_posts();
    assert_eq!(*delivered.lock().unwrap(), Some((Ok(42), thread::current().id())));
    assert_eq!(manual.queued_jobs(), 0);
    assert_eq!(manual.queued_posts(), 0);
}

#[test]
fn run_async_with_a_dropped_result_is_harmless() {
    let manual = Manual::new();
    let dispatcher = manual.dispatcher();
    run_async(&dispatcher, || String::from("ignored"), |_| {});
    manual.run_jobs();
    manual.run_posts();
    assert_eq!(manual.queued_jobs(), 0);
    assert_eq!(manual.queued_posts(), 0);
}

#[test]
fn a_panicking_work_gives_worker_panic_once() {
    let manual = Manual::new();
    let dispatcher = manual.dispatcher();
    let delivered: Arc<Mutex<Vec<(bool, thread::ThreadId)>>> = Arc::default();

    let delivered_in = Arc::clone(&delivered);
    run_async(
        &dispatcher,
        || -> u32 { panic!("synthetic worker failure") },
        move |result| {
            let panicked = matches!(result, Err(WorkerPanic { .. }));
            delivered_in.lock().unwrap().push((panicked, thread::current().id()));
        },
    );

    // `run_jobs` joins each job thread and fails the test when one panicked:
    // it returns only when the panic stayed inside `run_async`.
    let job_threads = manual.run_jobs();
    assert_eq!(job_threads.len(), 1);
    assert_ne!(job_threads[0], thread::current().id());
    assert_eq!(manual.queued_posts(), 1, "the failure is posted to the UI thread");
    assert!(delivered.lock().unwrap().is_empty());

    manual.run_posts();
    assert_eq!(*delivered.lock().unwrap(), vec![(true, thread::current().id())]);
    assert_eq!(manual.queued_posts(), 0);
}

#[test]
fn a_returning_work_gives_ok() {
    let manual = Manual::new();
    let dispatcher = manual.dispatcher();
    let delivered: Arc<Mutex<Vec<Result<u8, WorkerPanic>>>> = Arc::default();
    let delivered_in = Arc::clone(&delivered);
    run_async(&dispatcher, || 7_u8, move |result| delivered_in.lock().unwrap().push(result));
    manual.run_jobs();
    manual.run_posts();
    assert_eq!(*delivered.lock().unwrap(), vec![Ok(7)]);
}
