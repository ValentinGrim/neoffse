//! Helpers for the FF II editor tests: the registry that opens the synthetic
//! FF II slot, a machine that holds it, an open editor and a window over it.

use std::sync::Arc;

use neoffse::Registry;
use neoffse::data::Language;
use neoffse::pr::ff2::Ff2Format;

use super::{Editing, ID_1, Site, Ui, editing_in, ff2_slot_bytes, open_editor_in, slot_file, steam_slot};

/// The FF I format and the FF II format, English names.
pub(crate) fn ff2_registry() -> Arc<Registry> {
    super::registry_with(vec![Arc::new(Ff2Format::with_language(Language::En))])
}

/// A machine whose Proton folder of FF II holds the synthetic slot 1.
pub(crate) fn ff2_site() -> Site {
    let site = Site::new();
    // GAMES[1] is FF II.
    steam_slot(&site, 1, ID_1, slot_file(1), &ff2_slot_bytes(1));
    site
}

/// The editor view model over the synthetic FF II slot, run to the end.
pub(crate) fn ff2_editing(site: &Site) -> Editing {
    editing_in(site, ff2_registry())
}

/// A window on the editor page of the synthetic FF II slot.
pub(crate) fn ff2_ui(site: &Site) -> Ui {
    open_editor_in(site, ff2_registry())
}
