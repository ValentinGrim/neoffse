//! The shared summary reads a corps of any length: the five rows of FF IV
//! (`docs/formats/pixel-remaster.md#party`), none and one, through the FF I
//! model that calls it.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::SlotData;
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff1::{Ff1Model, GAME};

const CORPS: [Segment<'static>; 5] = [
    Segment::Key("userData"),
    Segment::Doc,
    Segment::Key("corpsList"),
    Segment::Doc,
    Segment::Key("target"),
];

fn standard() -> Document {
    Document::parse(&SlotData::standard().render()).unwrap()
}

fn party_of(document: Document) -> Vec<String> {
    let mut model = Ff1Model::new(document, neoffse_data::tables_for(GAME).unwrap());
    model.summary().unwrap().party
}

fn entry(character: u32) -> Scalar {
    Scalar::Text(format!("{{\"id\":1,\"characterId\":{character}}}"))
}

#[test]
fn a_corps_of_five_rows_gives_five_names() {
    let mut document = standard();
    assert_eq!(document.array_len(&CORPS).unwrap(), 4);
    document.insert(&CORPS, 4, &entry(2)).unwrap();
    assert_eq!(document.array_len(&CORPS).unwrap(), 5);
    assert_eq!(party_of(document), ["Aria", "Bram", "Cleo", "Zoé", "Bram"]);
}

#[test]
fn a_fifth_row_that_names_no_character_is_skipped() {
    let mut document = standard();
    document.insert(&CORPS, 4, &entry(9)).unwrap();
    assert_eq!(party_of(document), ["Aria", "Bram", "Cleo", "Zoé"]);
}

#[test]
fn an_empty_corps_gives_an_empty_party_and_no_error() {
    let mut document = standard();
    for _ in 0..4 {
        document.remove(&CORPS, 0).unwrap();
    }
    assert_eq!(document.array_len(&CORPS).unwrap(), 0);
    assert_eq!(party_of(document).len(), 0);
}

#[test]
fn a_corps_of_one_row_gives_one_name() {
    let mut document = standard();
    for _ in 0..3 {
        document.remove(&CORPS, 1).unwrap();
    }
    assert_eq!(document.array_len(&CORPS).unwrap(), 1);
    assert_eq!(party_of(document), ["Aria"]);
}
