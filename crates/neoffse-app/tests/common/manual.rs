//! A dispatcher that runs nothing until the test says so.

use std::mem;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, ThreadId};

use neoffse_app::dispatch::{Dispatcher, Job};

/// Two queues: `spawn` fills the jobs, `post` fills the UI tasks.
#[derive(Default)]
pub(crate) struct Manual {
    jobs: Mutex<Vec<Job>>,
    posts: Mutex<Vec<Job>>,
}

fn take(queue: &Mutex<Vec<Job>>) -> Vec<Job> {
    mem::take(&mut *queue.lock().unwrap_or_else(PoisonError::into_inner))
}

impl Manual {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// The same dispatcher, typed for the view models.
    pub(crate) fn dispatcher(self: &Arc<Self>) -> Arc<dyn Dispatcher> {
        Arc::<Self>::clone(self)
    }

    pub(crate) fn queued_jobs(&self) -> usize {
        self.jobs.lock().unwrap_or_else(PoisonError::into_inner).len()
    }

    pub(crate) fn queued_posts(&self) -> usize {
        self.posts.lock().unwrap_or_else(PoisonError::into_inner).len()
    }

    /// Runs each queued job on its own thread and waits for it. Returns the
    /// ids of the threads used.
    pub(crate) fn run_jobs(&self) -> Vec<ThreadId> {
        take(&self.jobs)
            .into_iter()
            .map(|job| {
                let handle = thread::spawn(job);
                let id = handle.thread().id();
                handle.join().expect("a job panicked");
                id
            })
            .collect()
    }

    /// Runs the queued UI tasks on the calling thread.
    pub(crate) fn run_posts(&self) {
        for task in take(&self.posts) {
            task();
        }
    }

    /// Alternates the two queues until both are empty.
    pub(crate) fn run_all(&self) {
        while self.queued_jobs() + self.queued_posts() > 0 {
            self.run_jobs();
            self.run_posts();
        }
    }
}

impl Dispatcher for Manual {
    fn spawn(&self, job: Job) {
        self.jobs.lock().unwrap_or_else(PoisonError::into_inner).push(job);
    }

    fn post(&self, task: Job) {
        self.posts.lock().unwrap_or_else(PoisonError::into_inner).push(task);
    }
}
