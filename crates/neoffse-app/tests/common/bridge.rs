//! Helpers for the editor page on a `MainWindow`: open the first synthetic
//! save, read the bridge, count history entries.

use std::sync::PoisonError;

use neoffse_app::{EditorBridge, EditorData, FieldRow};
use slint::ComponentHandle;

use super::{Part, Site, Ui, field};

/// A window whose first slot is open on the editor page.
pub(crate) fn open_editor(site: &Site) -> Ui {
    let ui = Ui::scanned(site, None);
    let key = ui.first_slots()[0].key;
    ui.window.invoke_slot_clicked(key);
    ui.manual.run_all();
    assert!(ui.window.get_editor_open(), "the save opens");
    ui
}

impl Ui {
    /// The data the screens read.
    pub(crate) fn bridge_data(&self) -> EditorData {
        self.window.global::<EditorBridge>().get_data()
    }

    /// The history entries of the open session.
    pub(crate) fn pending(&self) -> usize {
        let session = self.app.state.session().expect("a save is open");
        let guard = session.lock().unwrap_or_else(PoisonError::into_inner);
        guard.pending_count()
    }

    /// Sends `text` to the field `key`, as a commit from the screen.
    pub(crate) fn commit(&self, key: i32, text: &str) {
        self.window.global::<EditorBridge>().invoke_field_text_committed(key, text.into());
    }

    pub(crate) fn gil(&self) -> FieldRow {
        field(&self.bridge_data(), Part::Inventory, "Gil", "Gil")
    }

    pub(crate) fn level(&self) -> FieldRow {
        field(&self.bridge_data(), Part::Party, "Aria", "Level")
    }
}
