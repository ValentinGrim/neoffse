//! `Ff1Document::summary`: the slot falls back to the file name when the
//! top-level `id` gives none.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_format::{one_file, open, sav, slot_name};
use common::ff1_save::SlotData;
use neoffse_core::{SaveDocument, SaveSlot};
use neoffse_format_pr::container::Container;

const ID_PAIR: &str = "\"id\":1,";

/// The standard slot with its `id` pair removed, or set to 99.
fn variants() -> [(&'static str, String); 2] {
    let text = SlotData::standard().render();
    assert!(text.starts_with(&format!("{{{ID_PAIR}")), "the id is the first pair");
    [
        ("absent", text.replacen(ID_PAIR, "", 1)),
        ("out of range", text.replacen(ID_PAIR, "\"id\":99,", 1)),
    ]
}

fn slot_under(name: &str, text: &str) -> Option<SaveSlot> {
    open(&one_file(name, sav(text, &Container::game()))).summary().unwrap().slot
}

#[test]
fn a_slot_without_a_usable_id_takes_the_slot_of_the_file_name() {
    for (case, text) in variants() {
        for number in [1, 7, 20] {
            assert_eq!(
                slot_under(slot_name(number), &text),
                Some(SaveSlot::Numbered(number)),
                "{case} under slot {number}"
            );
        }
        assert_eq!(slot_under(slot_name(21), &text), Some(SaveSlot::Auto), "{case} under slot 21");
        assert_eq!(slot_under(slot_name(22), &text), Some(SaveSlot::Quick), "{case} under slot 22");
    }
}

#[test]
fn a_slot_without_a_usable_id_under_another_name_has_no_slot() {
    for (case, text) in variants() {
        assert_eq!(slot_under("copy.sav", &text), None, "{case}");
    }
}

#[test]
fn a_usable_id_wins_over_the_file_name() {
    let text = SlotData::standard().render();
    assert_eq!(slot_under(slot_name(7), &text), Some(SaveSlot::Numbered(1)));
    assert_eq!(slot_under("copy.sav", &text), Some(SaveSlot::Numbered(1)));
}
