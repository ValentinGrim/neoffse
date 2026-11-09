//! Files for the registry tests that hold two games: a slot with a chosen
//! vehicle count, and a registry of the real FF I and FF II formats.

use std::sync::Arc;

use neoffse::Registry;
use neoffse::core::Game;
use neoffse::data::Language;
use neoffse::pr::OpenError;
use neoffse::pr::container::{self, Container};
use neoffse::pr::document::{Document, Segment};
use neoffse::pr::ff1::Ff1Format;
use neoffse::pr::ff2::Ff2Format;
use neoffse::pr::games::identify;

/// FF I and FF II, the two real formats. A file built from the FF I fixture
/// with 7 vehicles opens through the FF II format because `open` runs the
/// game rule and the shape gate only; a test that reads the opened file
/// further needs a real FF II slot.
pub(crate) fn ff2_registry() -> Registry {
    Registry::from_formats(vec![
        Arc::new(Ff1Format::with_language(Language::En)),
        Arc::new(Ff2Format::with_language(Language::En)),
    ])
}

const TARGET: [Segment<'static>; 5] = [
    Segment::Key("userData"),
    Segment::Doc,
    Segment::Key("ownedTransportationList"),
    Segment::Doc,
    Segment::Key("target"),
];

/// One vehicle entry, as the game writes an unplaced one.
const VEHICLE: &str = r#"{"position":{"x":1000.0,"y":1000.0,"z":-1000.0},"direction":0,"id":1,"mapId":-1,"enable":false,"timeStampTicks":0}"#;

/// The vehicle counts that name a game
/// (`docs/formats/pixel-remaster.md#telling-the-games-apart`).
const COUNTS: [(usize, Game); 6] = [
    (4, Game::Ff1Pr),
    (7, Game::Ff2Pr),
    (14, Game::Ff3Pr),
    (11, Game::Ff4Pr),
    (19, Game::Ff5Pr),
    (8, Game::Ff6Pr),
];

/// A JSON document as the string value that holds it: quoted, with `\` and
/// `"` escaped (`docs/formats/pixel-remaster.md#documents-inside-strings`).
fn embed(document: &str) -> String {
    format!("\"{}\"", document.replace('\\', "\\\\").replace('"', "\\\""))
}

/// The text of the FF I fixture with `ownedTransportationList.target`
/// resized to `count` entries. The game rule reads the count back before the
/// text is returned: it must name the game of `count`, or no game.
fn text_with_vehicles(count: usize) -> String {
    let json = neoffse_testkit::read_fixture("pr/ff1-slot.json").unwrap();
    let mut document = Document::parse(&String::from_utf8(json).unwrap()).unwrap();
    let have = document.array_len(&TARGET).unwrap();
    for _ in have..count {
        document.insert_raw(&TARGET, 0, &embed(VEHICLE)).unwrap();
    }
    for _ in count..have {
        document.remove(&TARGET, 0).unwrap();
    }
    let text = document.to_json();
    let found = identify(&mut Document::parse(&text).unwrap());
    match COUNTS.iter().find(|(owned, _)| *owned == count) {
        Some((_, game)) => assert_eq!(found.unwrap(), *game, "{count} vehicles"),
        None => assert!(matches!(found, Err(OpenError::OtherGame { found: None, .. })), "{count} vehicles"),
    }
    text
}

/// The synthetic slot with `count` vehicles, in the game's container.
pub(crate) fn slot_with_vehicles(count: usize) -> Vec<u8> {
    container::encode(&text_with_vehicles(count), &Container::game()).unwrap()
}

/// Like [`slot_with_vehicles`], with the first `from` of the slot text
/// replaced by `to`. The replace must hit, or the helper panics.
pub(crate) fn slot_with_text_replaced(count: usize, from: &str, to: &str) -> Vec<u8> {
    let text = text_with_vehicles(count);
    let replaced = text.replacen(from, to, 1);
    assert_ne!(replaced, text, "{from} is not in the slot text");
    container::encode(&replaced, &Container::game()).unwrap()
}

/// The synthetic FF II slot of `fixtures/pr/ff2-slot.json`, in the game's
/// container.
pub(crate) fn ff2_slot_bytes() -> Vec<u8> {
    let json = neoffse_testkit::read_fixture("pr/ff2-slot.json").unwrap();
    container::encode(&String::from_utf8(json).unwrap(), &Container::game()).unwrap()
}
