//! Remembers the size Home had when the editor opened, so the window returns
//! to it when the editor closes.
//!
//! The editor may open a window larger than Home (its own minimum) and the
//! user may resize it there. The window belongs to the shell, so this memory
//! is glue and no screen logic (`docs/architecture.md#ui-architecture`).
//!
//! Invariants:
//! - The size is kept in logical pixels: a move to a monitor with another
//!   scale factor restores the same visible size.
//! - A maximised or fullscreen window records and restores nothing: the window
//!   manager owns its size.
//! - One record per opening. `restore` always forgets it, so the next opening
//!   takes a fresh one.
//! - Two sizes are equal when both sides differ by less than half a pixel;
//!   an equal size calls no `set_size`.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use slint::{ComponentHandle, LogicalSize};

use crate::MainWindow;

/// Below this gap in logical pixels, two sizes are the same.
const TOLERANCE: f32 = 0.5;

/// The size Home had when the editor opened, in logical pixels. One record
/// per [`App`](crate::App); clones share it.
#[derive(Debug, Clone, Default)]
pub(crate) struct HomeSize {
    recorded: Arc<Mutex<Option<LogicalSize>>>,
}

impl HomeSize {
    /// Records the size of `window` as the Home size, replacing an older
    /// record; the record is empty while the window is maximised or
    /// fullscreen. Does nothing while the editor is already open (the first
    /// record of an opening wins).
    pub(crate) fn remember(&self, window: &MainWindow) {
        if window.get_editor_open() {
            return;
        }
        *self.lock() = if managed(window) { None } else { Some(logical_size(window)) };
    }

    /// Puts `window` back to the recorded size when it differs from it, then
    /// forgets the record. Does nothing without a record, and nothing while
    /// the window is maximised or fullscreen (the record is still forgotten).
    pub(crate) fn restore(&self, window: &MainWindow) {
        let Some(recorded) = self.lock().take() else { return };
        if managed(window) {
            return;
        }
        let now = logical_size(window);
        if (now.width - recorded.width).abs() >= TOLERANCE || (now.height - recorded.height).abs() >= TOLERANCE {
            window.window().set_size(recorded);
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<LogicalSize>> {
        self.recorded.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Whether the window manager decides the size of `window`.
fn managed(window: &MainWindow) -> bool {
    window.window().is_maximized() || window.window().is_fullscreen()
}

fn logical_size(window: &MainWindow) -> LogicalSize {
    window.window().size().to_logical(window.window().scale_factor())
}
