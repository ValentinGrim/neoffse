//! `Ff1Model::summary` against the synthetic slot.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::time::Duration;

use common::ff1_save::SlotData;
use neoffse_core::{FormatError, SaveSlot};
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff1::{Ff1Model, GAME};

const PLAY_TIME: [Segment<'static>; 3] = [Segment::Key("userData"), Segment::Doc, Segment::Key("playTime")];
const LOCATION: [Segment<'static>; 3] = [Segment::Key("userData"), Segment::Doc, Segment::Key("currentLocation")];

fn document(slot: &SlotData) -> Document {
    Document::parse(&slot.render()).unwrap()
}

fn model_of(document: Document) -> Ff1Model {
    Ff1Model::new(document, neoffse_data::tables_for(GAME).unwrap())
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
    assert_eq!(summary.party, ["Aria", "Bram", "Cleo", "Zoé"]);
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
    document.set(&corps_id(3), &Scalar::Int(9)).unwrap();
    let summary = model_of(document).summary().unwrap();
    assert_eq!(summary.party, ["Aria", "Zoé", "Cleo"]);
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
    assert_eq!(summary.party.len(), 4);
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
    assert_eq!(summary.party, ["Aria", "", "Cleo", "Zoé"]);
}

#[test]
fn a_character_listed_twice_in_the_corps_gives_the_name_twice() {
    let mut document = document(&SlotData::standard());
    document.set(&corps_id(1), &Scalar::Int(1)).unwrap();
    let summary = model_of(document).summary().unwrap();
    assert_eq!(summary.party, ["Aria", "Aria", "Cleo", "Zoé"]);
}

#[test]
fn a_corps_entry_with_a_text_character_id_is_skipped() {
    let mut document = document(&SlotData::standard());
    document.set(&corps_id(1), &Scalar::Text("2".to_owned())).unwrap();
    let summary = model_of(document).summary().unwrap();
    assert_eq!(summary.party, ["Aria", "Cleo", "Zoé"]);
}
