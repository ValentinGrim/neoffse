//! Helpers shared by the test crates. Each crate declares `mod common;`:
//! Cargo does not build this folder as a test target.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(unused_imports, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod backups;
mod bridge;
mod editor;
mod geometry;
mod keys;
mod manual;
mod open;
mod restoring;
mod rig;
mod save_proof;
mod saving;
mod site;
mod slots;
mod window;

use std::sync::Arc;

use neoffse::Registry;
use neoffse::data::Language;
use neoffse_app::home::HomeViewModel;
use neoffse_app::{GameRow, SlotRow};
use slint::{Model, ModelRc};

pub(crate) use backups::{T0, listing, make_backup, rewrite_manifest};
pub(crate) use bridge::open_editor;
pub(crate) use editor::{
    Editing, FakeLocks, Part, all_fields, count_of, editing, editing_site, field, fields, item_label, list, lists, page, pages, review_row, row_of, rows,
    section, spare_weapon,
};
pub(crate) use geometry::{
    BAND, Card, Rect, SLACK, assert_size, bands_of, cards_ui, first_element, first_rect, font_body, logical, rects, resize, row_height, texts_of, visible_cards,
};
pub(crate) use manual::Manual;
pub(crate) use open::{FixedPicker, Flow, Outcomes, PrivateSlot, private_slot_copies};
pub(crate) use restoring::{Ended, Paged, Restoring, restore_page};
pub(crate) use rig::{Machine, Rig, backup_count, break_data_dir, closeable_rig, gil_in, newest_created, rig, scripted_probe, session_in};
pub(crate) use save_proof::{Step, assert_same};
pub(crate) use saving::{Saving, saving, saving_over};
pub(crate) use site::{ID_1, ID_2, Site, documents_folder, documents_slot, environment, install, steam_folder, steam_slot};
pub(crate) use slots::{slot_bytes, slot_file};
pub(crate) use window::Ui;

/// The value of `result`, or a panic with `what` alone. The `Err` is dropped
/// because its text can quote a path or a value of a private save.
#[track_caller]
pub(crate) fn must<T, E>(result: Result<T, E>, what: &str) -> T {
    result.unwrap_or_else(|_| panic!("{what}"))
}

/// The value of `option`, or a panic with `what`.
#[track_caller]
pub(crate) fn must_some<T>(option: Option<T>, what: &str) -> T {
    match option {
        Some(value) => value,
        None => panic!("{what}"),
    }
}

pub(crate) fn registry() -> Arc<Registry> {
    Arc::new(Registry::new(Language::En))
}

/// A view model over `site`, scanned to the end, filter on.
pub(crate) fn scanned(site: &Site) -> HomeViewModel {
    let home = HomeViewModel::new(registry(), environment(site));
    let manual = Manual::new();
    home.start_scan(&manual.dispatcher(), || {});
    manual.run_all();
    home
}

/// The rows of every game, the filter off. Restores nothing: the filter stays off.
pub(crate) fn all_rows(home: &HomeViewModel) -> Vec<GameRow> {
    home.set_owned_only(false);
    home.rows()
}

pub(crate) fn items<T: Clone + 'static>(model: &ModelRc<T>) -> Vec<T> {
    (0..model.row_count()).filter_map(|row| model.row_data(row)).collect()
}

pub(crate) fn slots_of(row: &GameRow) -> Vec<SlotRow> {
    items(&row.slots)
}
