//! `Ff1Document::summary`: the slot falls back to the file name when the
//! top-level `id` gives none.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::SlotData;
use common::pr_files::slot_name;
use common::{id_variants, slot_under};
use neoffse_core::SaveSlot;
use neoffse_format_pr::ff1::Ff1Format;

#[test]
fn a_slot_without_a_usable_id_takes_the_slot_of_the_file_name() {
    for (case, text) in id_variants(&SlotData::standard().render()) {
        for number in [1, 7, 20] {
            assert_eq!(
                slot_under(&Ff1Format::new(), slot_name(number), &text),
                Some(SaveSlot::Numbered(number)),
                "{case} under slot {number}"
            );
        }
        assert_eq!(
            slot_under(&Ff1Format::new(), slot_name(21), &text),
            Some(SaveSlot::Auto),
            "{case} under slot 21"
        );
        assert_eq!(
            slot_under(&Ff1Format::new(), slot_name(22), &text),
            Some(SaveSlot::Quick),
            "{case} under slot 22"
        );
    }
}

#[test]
fn a_slot_without_a_usable_id_under_another_name_has_no_slot() {
    for (case, text) in id_variants(&SlotData::standard().render()) {
        assert_eq!(slot_under(&Ff1Format::new(), "copy.sav", &text), None, "{case}");
    }
}

#[test]
fn a_usable_id_wins_over_the_file_name() {
    let text = SlotData::standard().render();
    assert_eq!(slot_under(&Ff1Format::new(), slot_name(7), &text), Some(SaveSlot::Numbered(1)));
    assert_eq!(slot_under(&Ff1Format::new(), "copy.sav", &text), Some(SaveSlot::Numbered(1)));
}
