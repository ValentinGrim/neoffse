//! Synthetic slot files.

use neoffse::pr::container::{self, Container};
use neoffse::pr::document::{Document, Scalar, Segment};
use neoffse::pr::slots::SLOT_FILES;

/// The synthetic FF I slot with its top-level `id` set to `id`, in the
/// game's container.
pub(crate) fn slot_bytes(id: i64) -> Vec<u8> {
    let json = neoffse_testkit::read_fixture("pr/ff1-slot.json").unwrap();
    let mut document = Document::parse(&String::from_utf8(json).unwrap()).unwrap();
    document.set(&[Segment::Key("id")], &Scalar::Int(id)).unwrap();
    container::encode(&document.to_json(), &Container::game()).unwrap()
}

/// The 44-character file name of the slot with JSON id `id`.
pub(crate) fn slot_file(id: u32) -> &'static str {
    SLOT_FILES.iter().find(|file| file.id == id).unwrap().name
}

/// The vehicle list of the synthetic slot.
const VEHICLES: [Segment<'static>; 5] = [
    Segment::Key("userData"),
    Segment::Doc,
    Segment::Key("ownedTransportationList"),
    Segment::Doc,
    Segment::Key("target"),
];

fn slot_text() -> String {
    let json = neoffse_testkit::read_fixture("pr/ff1-slot.json").unwrap();
    let document = Document::parse(&String::from_utf8(json).unwrap()).unwrap();
    document.to_json()
}

/// A slot with the key only FF VI saves hold, in the game's container: a real
/// save of a game the registry cannot open.
pub(crate) fn other_game_bytes() -> Vec<u8> {
    let text = slot_text();
    let marked = text.replacen(r#"\"moveCount\":10"#, r#"\"otherPartyDataList\":[],\"moveCount\":10"#, 1);
    assert_ne!(marked, text, "the marker is not in the slot text");
    container::encode(&marked, &Container::game()).unwrap()
}

/// A slot with no vehicle: a count no game owns.
pub(crate) fn no_game_bytes() -> Vec<u8> {
    let mut document = Document::parse(&slot_text()).unwrap();
    for _ in 0..4 {
        document.remove(&VEHICLES, 0).unwrap();
    }
    container::encode(&document.to_json(), &Container::game()).unwrap()
}
