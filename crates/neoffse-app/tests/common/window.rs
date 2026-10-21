//! A `MainWindow` wired to the view models over one synthetic machine, with
//! the manual dispatcher. The window never shows.

use std::path::PathBuf;
use std::sync::Arc;

use neoffse_app::{App, FilePicker, GameRow, MainWindow, SlotRow, connect_home};

use super::{FixedPicker, Manual, Site, environment, items, registry, slots_of};

pub(crate) struct Ui {
    pub(crate) window: MainWindow,
    pub(crate) app: App,
    pub(crate) manual: Arc<Manual>,
    pub(crate) picker: Arc<FixedPicker>,
}

impl Ui {
    /// Installs the testing backend for this thread, builds the window and
    /// connects it. The first scan is queued, not run.
    pub(crate) fn new(site: &Site, pick: Option<PathBuf>) -> Self {
        i_slint_backend_testing::init_no_event_loop();
        let manual = Manual::new();
        let picker = FixedPicker::new(pick);
        let app = App::new(registry(), environment(site), Arc::clone(&picker) as Arc<dyn FilePicker>, manual.dispatcher());
        let window = MainWindow::new().unwrap();
        connect_home(&window, &app);
        Self { window, app, manual, picker }
    }

    /// Like `new`, then the first scan runs to the end.
    pub(crate) fn scanned(site: &Site, pick: Option<PathBuf>) -> Self {
        let ui = Self::new(site, pick);
        ui.manual.run_all();
        ui
    }

    pub(crate) fn rows(&self) -> Vec<GameRow> {
        items(&self.window.get_games())
    }

    /// The slots of the first game row.
    pub(crate) fn first_slots(&self) -> Vec<SlotRow> {
        slots_of(&self.rows()[0])
    }
}
