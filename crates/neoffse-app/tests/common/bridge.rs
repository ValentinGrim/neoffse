//! Helpers for the editor page on a `MainWindow`: open the first synthetic
//! save, read the bridge, count history entries.

use std::sync::PoisonError;

use neoffse_app::{EditorBridge, EditorData, FieldRow};
use slint::ComponentHandle;

use super::{Part, Site, Ui, field, fields, page};

/// A window whose first slot is open on the editor page.
pub(crate) fn open_editor(site: &Site) -> Ui {
    let ui = Ui::scanned(site, None);
    ui.open_first_slot();
    ui
}

impl Ui {
    /// The data the screens read.
    pub(crate) fn bridge_data(&self) -> EditorData {
        self.window.global::<EditorBridge>().get_data()
    }

    /// The editor bridge of the window.
    pub(crate) fn bridge(&self) -> EditorBridge<'_> {
        self.window.global::<EditorBridge>()
    }

    /// The first field of the Magic page of the first character.
    pub(crate) fn spell(&self) -> FieldRow {
        let data = self.bridge_data();
        fields(&page(&data, Part::Magic, "Aria")).into_iter().next().expect("a spell slot")
    }

    /// The count field of the first equipment slot of the first character.
    pub(crate) fn slot_count(&self) -> FieldRow {
        field(&self.bridge_data(), Part::Party, "Aria", "Count in slot 1")
    }

    /// Whether a save is open.
    pub(crate) fn session_open(&self) -> bool {
        self.app.state.is_open()
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
