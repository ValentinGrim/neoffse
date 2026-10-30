//! A `MainWindow` wired to the view models over one synthetic machine, with
//! the manual dispatcher. Nothing shows until a test calls `show()`, and the
//! testing backend has no screen.

use std::path::PathBuf;
use std::sync::Arc;

use neoffse::data::Language;
use neoffse_app::{
    AboutDialog, App, FilePicker, GameRow, MainWindow, RestoreBridge, SlotRow, connect_about, connect_close, connect_editor, connect_expert, connect_history,
    connect_home, connect_quit, connect_restore, connect_review, connect_save, new_about_dialog,
};

use slint::ComponentHandle;
use slint::platform::WindowEvent;

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
        let app = App::new(
            registry(),
            Language::En,
            environment(site),
            Arc::clone(&picker) as Arc<dyn FilePicker>,
            manual.dispatcher(),
        );
        let window = MainWindow::new().unwrap();
        connect_home(&window, &app);
        connect_editor(&window, &app);
        connect_history(&window, &app);
        connect_review(&window, &app);
        connect_save(&window, &app);
        connect_restore(&window, &app);
        connect_expert(&window, &app);
        Self { window, app, manual, picker }
    }

    /// Like `new`, with About and Quit connected and the close handler
    /// installed after them, as `run` does.
    pub(crate) fn with_about(site: &Site) -> (Self, AboutDialog) {
        let ui = Self::new(site, None);
        let about = new_about_dialog().unwrap();
        connect_about(&ui.window, &about);
        connect_quit(&ui.window);
        connect_close(&ui.window, &about, &ui.app);
        ui.manual.run_all();
        (ui, about)
    }

    /// Like `new`, then the first scan runs to the end.
    pub(crate) fn scanned(site: &Site, pick: Option<PathBuf>) -> Self {
        let ui = Self::new(site, pick);
        ui.manual.run_all();
        ui
    }

    /// The Restore bridge of the window.
    pub(crate) fn restore_bridge(&self) -> RestoreBridge<'_> {
        self.window.global::<RestoreBridge>()
    }

    pub(crate) fn rows(&self) -> Vec<GameRow> {
        items(&self.window.get_games())
    }

    /// The slots of the first game row.
    pub(crate) fn first_slots(&self) -> Vec<SlotRow> {
        slots_of(&self.rows()[0])
    }

    /// Clicks the first slot of the first game and runs the open to the end.
    pub(crate) fn open_first_slot(&self) {
        let key = self.first_slots()[0].key;
        self.window.invoke_slot_clicked(key);
        self.manual.run_all();
        assert!(self.window.get_editor_open(), "the save opens");
    }

    /// The window manager's close request, as the close button sends it.
    pub(crate) fn request_close(&self) {
        self.window.window().dispatch_event(WindowEvent::CloseRequested);
    }
}
