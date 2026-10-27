//! Where work runs: off the UI thread, and back on it.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

/// A unit of work handed to a thread.
pub type Job = Box<dyn FnOnce() + Send + 'static>;

/// Runs jobs off the UI thread and tasks on it.
pub trait Dispatcher: Send + Sync + 'static {
    /// Runs `job` on a thread other than the UI thread. The call returns at
    /// once.
    fn spawn(&self, job: Job);
    /// Runs `task` on the UI thread.
    fn post(&self, task: Job);
}

/// The production dispatcher: one `std::thread` per job, and
/// `slint::invoke_from_event_loop` for the way back.
#[derive(Debug, Clone, Copy, Default)]
pub struct EventLoopDispatcher;

impl Dispatcher for EventLoopDispatcher {
    fn spawn(&self, job: Job) {
        // A failed thread start (OS resource exhaustion) drops the job and its reply: a save or restore
        // then stays marked working and the window close stays vetoed.
        let _ = std::thread::Builder::new().name("neoffse-worker".into()).spawn(job);
    }

    fn post(&self, task: Job) {
        // An error means the event loop has ended, so the task has no UI left to serve.
        let _ = slint::invoke_from_event_loop(task);
    }
}

/// A worker closure panicked.
///
/// [`run_async`] catches the panic and hands this value to the UI-thread
/// step, so a screen can clear its busy flag instead of waiting for ever. The
/// panic payload is dropped: it can quote a path or save content, so it never
/// reaches a label or a log.
///
/// Catching relies on unwinding. A profile that sets `panic = "abort"` ends
/// the process on the first worker panic and this type never appears. The
/// workspace sets no such profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct WorkerPanic;

/// Runs `work` off the UI thread, then `then(result)` on it.
///
/// `result` is `Err(WorkerPanic)` when `work` panicked, and the worker thread
/// then ends normally. The call returns before `work` starts.
pub fn run_async<T, W, D>(dispatcher: &Arc<dyn Dispatcher>, work: W, then: D)
where
    T: Send + 'static,
    W: FnOnce() -> T + Send + 'static,
    D: FnOnce(Result<T, WorkerPanic>) + Send + 'static,
{
    let back = Arc::clone(dispatcher);
    dispatcher.spawn(Box::new(move || {
        let result = catch_unwind(AssertUnwindSafe(work)).map_err(|_payload| WorkerPanic);
        back.post(Box::new(move || then(result)));
    }));
}
