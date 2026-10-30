//! The process probe shared by the save and restore view models.

use std::sync::{Arc, Mutex, PoisonError};

use neoffse::io::process;

/// The process scan a save or a restore runs under `Running::Refuse`: the
/// executable names of the game in, the names found running out.
pub(crate) type Probe = Arc<dyn Fn(&[&'static str]) -> Vec<&'static str> + Send + Sync>;

/// A replaceable [`Probe`]. A new slot holds `neoffse::io::process::running`.
pub(crate) struct ProbeSlot(Mutex<Probe>);

impl ProbeSlot {
    pub(crate) fn new() -> Self {
        Self(Mutex::new(Arc::new(process::running)))
    }

    /// Replaces the probe for the next save or restore.
    pub(crate) fn set(&self, probe: impl Fn(&[&'static str]) -> Vec<&'static str> + Send + Sync + 'static) {
        *self.lock() = Arc::new(probe);
    }

    /// The probe a worker clones before it leaves the calling thread.
    pub(crate) fn get(&self) -> Probe {
        Arc::clone(&self.lock())
    }

    // The slot holds one `Arc`, which a panic cannot leave half written, so a
    // poisoned lock is safe to use.
    fn lock(&self) -> std::sync::MutexGuard<'_, Probe> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for ProbeSlot {
    fn default() -> Self {
        Self::new()
    }
}
