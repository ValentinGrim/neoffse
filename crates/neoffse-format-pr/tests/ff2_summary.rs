//! `Ff2Model::summary` and `Ff2Document::summary` against the synthetic slot.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::time::Duration;

use common::ff1_format::{one_file, sav, slot_name};
use common::ff2_save::{SlotData, tables};
use neoffse_core::{FormatError, SaveDocument, SaveSlot};
use neoffse_format_pr::container::Container;
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Format, Ff2Model, GAME};

const PLAY_TIME: [Segment<'static>; 3] = [Segment::Key("userData"), Segment::Doc, Segment::Key("playTime")];
const LOCATION: [Segment<'static>; 3] = [Segment::Key("userData"), Segment::Doc, Segment::Key("currentLocation")];

fn document(slot: &SlotData) -> Document {
    Document::parse(&slot.render()).unwrap()
}

fn model_of(document: Document) -> Ff2Model {
    Ff2Model::new(document, tables())
}

fn party_of(corps: &[u32]) -> Vec<String> {
    let slot = SlotData {
        corps: corps.to_vec(),
        ..SlotData::standard()
    };
    model_of(document(&slot)).summary().unwrap().party
}

fn corps_id(index: usize) -> [Segment<'static>; 8] {
    [
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("corpsList"),
        Segment::Doc,
        Segment::Key("target"),
        Segment::Index(index),
        Segment::Doc,
        Segment::Key("characterId"),
    ]
}

#[test]
fn summary_reads_slot_play_time_place_and_party() {
    let summary = model_of(document(&SlotData::standard())).summary().unwrap();
    assert_eq!(summary.game, GAME);
    assert_eq!(summary.slot, Some(SaveSlot::Numbered(1)));
    assert_eq!(summary.play_time, Some(Duration::from_secs_f64(13_621.814_333_377_138)));
    assert_eq!(summary.location.as_deref(), Some("Invented Plains"));
    assert_eq!(summary.party, ["Ivo", "Mara", "Tane"]);
}

#[test]
fn an_empty_seat_is_skipped() {
    // The standard corps is [1, 2, 3, 0]: the fourth seat holds no character.
    assert_eq!(party_of(&[1, 2, 3, 0]), ["Ivo", "Mara", "Tane"]);
    assert_eq!(party_of(&[0, 1, 0, 2]), ["Ivo", "Mara"]);
    assert_eq!(party_of(&[0, 0, 0, 0]).len(), 0);
    assert_eq!(party_of(&[]).len(), 0);
}

#[test]
fn a_full_party_and_a_fifth_seat_list_every_name() {
    assert_eq!(party_of(&[1, 2, 3, 4]), ["Ivo", "Mara", "Tane", "Élan"]);
    assert_eq!(party_of(&[1, 2, 3, 4, 2]), ["Ivo", "Mara", "Tane", "Élan", "Mara"]);
}

#[test]
fn current_location_wins_when_it_is_set() {
    let mut document = document(&SlotData::standard());
    document.set(&LOCATION, &Scalar::Text("Invented Coast".to_owned())).unwrap();
    let summary = model_of(document).summary().unwrap();
    assert_eq!(summary.location.as_deref(), Some("Invented Coast"));
}

#[test]
fn the_party_follows_the_list_order_and_skips_dangling_ids() {
    let mut document = document(&SlotData::standard());
    document.set(&corps_id(1), &Scalar::Int(4)).unwrap();
    document.set(&corps_id(2), &Scalar::Int(9)).unwrap();
    let summary = model_of(document).summary().unwrap();
    assert_eq!(summary.party, ["Ivo", "Élan"]);
}

#[test]
fn play_time_forms() {
    let cases: [(&str, Option<Duration>); 5] = [
        ("13621", Some(Duration::from_secs(13621))),
        ("-1", None),
        ("-1.5", None),
        ("\"soon\"", None),
        ("null", None),
    ];
    for (raw, expected) in cases {
        let mut document = document(&SlotData::standard());
        document.set_raw(&PLAY_TIME, raw).unwrap();
        let summary = model_of(document).summary().unwrap();
        assert_eq!(summary.play_time, expected, "playTime {raw}");
    }
}

#[test]
fn an_overflowing_play_time_is_a_decode_error() {
    let mut document = document(&SlotData::standard());
    document.set_raw(&PLAY_TIME, "1e400").unwrap();
    assert!(matches!(model_of(document).summary(), Err(FormatError::Decode { .. })));
}

#[test]
fn a_missing_key_gives_the_empty_value() {
    let text = SlotData::standard().render();
    let renamed = text.replace("\\\"currentArea\\\"", "\\\"currentAreX\\\"");
    assert_ne!(renamed, text);
    let summary = model_of(Document::parse(&renamed).unwrap()).summary().unwrap();
    assert_eq!(summary.location, None);
    assert_eq!(summary.slot, Some(SaveSlot::Numbered(1)));
    assert_eq!(summary.party.len(), 3);
}

#[test]
fn the_slot_is_none_outside_the_table() {
    for id in [23, 0] {
        let mut document = document(&SlotData::standard());
        document.set(&[Segment::Key("id")], &Scalar::Int(id)).unwrap();
        assert_eq!(model_of(document).summary().unwrap().slot, None, "id {id}");
    }
}

#[test]
fn a_document_without_user_data_gives_a_slot_only_summary() {
    let slot = SlotData {
        with_user_data: false,
        ..SlotData::standard()
    };
    let summary = model_of(document(&slot)).summary().unwrap();
    assert_eq!(summary.slot, Some(SaveSlot::Numbered(1)));
    assert_eq!(summary.party.len(), 0);
    assert_eq!(summary.play_time, None);
    assert_eq!(summary.location, None);
}

#[test]
fn an_empty_user_data_is_a_decode_error() {
    let mut document = document(&SlotData::standard());
    document.set_raw(&[Segment::Key("userData")], "\"\"").unwrap();
    assert!(matches!(model_of(document).summary(), Err(FormatError::Decode { .. })));
}

#[test]
fn summary_changes_nothing() {
    let slot = SlotData {
        spaced: true,
        ..SlotData::standard()
    };
    let mut model = model_of(document(&slot));
    let before = model.document().to_json();
    model.summary().unwrap();
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn an_empty_character_name_stays_empty_in_the_party() {
    let mut slot = SlotData::standard();
    slot.characters[1].name = String::new();
    let summary = model_of(document(&slot)).summary().unwrap();
    assert_eq!(summary.party, ["Ivo", "", "Tane"]);
}

#[test]
fn a_character_listed_twice_in_the_corps_gives_the_name_twice() {
    assert_eq!(party_of(&[1, 1, 3, 0]), ["Ivo", "Ivo", "Tane"]);
}

#[test]
fn a_corps_entry_with_a_text_character_id_is_skipped() {
    let mut document = document(&SlotData::standard());
    document.set(&corps_id(1), &Scalar::Text("2".to_owned())).unwrap();
    let summary = model_of(document).summary().unwrap();
    assert_eq!(summary.party, ["Ivo", "Tane"]);
}

#[test]
fn a_non_text_character_name_is_a_decode_error() {
    let mut document = document(&SlotData::standard());
    let name = common::character_path(0, &[Segment::Doc, Segment::Key("name")]);
    document.set_raw(&name, "7").unwrap();
    assert!(matches!(model_of(document).summary(), Err(FormatError::Decode { .. })));
}

mod slot_fallback {
    use super::*;

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
        let files = one_file(name, sav(text, &Container::game()));
        Ff2Format::new().open_document(&files).unwrap().summary().unwrap().slot
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
            assert_eq!(slot_under("copy.sav", &text), None, "{case} under another name");
        }
    }

    #[test]
    fn a_usable_id_wins_over_the_file_name() {
        let text = SlotData::standard().render();
        assert_eq!(slot_under(slot_name(7), &text), Some(SaveSlot::Numbered(1)));
        assert_eq!(slot_under("copy.sav", &text), Some(SaveSlot::Numbered(1)));
    }
}
