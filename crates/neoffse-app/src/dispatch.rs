//! Where work runs: off the UI thread, and back on it.

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
        // A failed thread start (OS resource exhaustion) loses the job: a known gap.
        let _ = std::thread::Builder::new().name("neoffse-worker".into()).spawn(job);
    }

    fn post(&self, task: Job) {
        // An error means the event loop has ended, so the task has no UI left to serve.
        let _ = slint::invoke_from_event_loop(task);
    }
}

/// Runs `work` off the UI thread, then `then(result)` on it.
///
/// The call returns before `work` starts.
pub fn run_async<T, W, D>(dispatcher: &Arc<dyn Dispatcher>, work: W, then: D)
where
    T: Send + 'static,
    W: FnOnce() -> T + Send + 'static,
    D: FnOnce(T) + Send + 'static,
{
    let back = Arc::clone(dispatcher);
    dispatcher.spawn(Box::new(move || {
        let result = work();
        back.post(Box::new(move || then(result)));
    }));
}
