//! A second real `SlotFormat` for the registry tests: the shared slot
//! pipeline, the real game rule and the real `OpenError`s, with a model that
//! reports FF II.

use std::sync::Arc;

use neoffse::Registry;
use neoffse::core::{
    CapPolicy, Capability, CollectionMeta, Edit, EditError, FieldId, FieldMeta, FormatError, Game, GameChange, RowId, SaveSummary, SaveView, Value,
};
use neoffse::data::{GameTables, Language};
use neoffse::pr::OpenError;
use neoffse::pr::container::{self, Container};
use neoffse::pr::document::{Document, Segment};
use neoffse::pr::ff1::{self, Ff1Format, Ff1Model};
use neoffse::pr::games::identify;
use neoffse::pr::shared::{SlotFormat, SlotModel};

/// An FF I model that reports FF II, so the registry meets a second real
/// `SlotFormat`. It reads an FF I document with the FF II tables: no FF II
/// format exists. A test may call `open`, `game`, `capabilities` and `rows`
/// on it; `summary`, `view` and edits are not valid on it.
#[derive(Clone)]
pub(crate) struct Ff2Stand {
    inner: Ff1Model,
    tables: &'static GameTables,
}

impl SlotModel for Ff2Stand {
    const GAME: Game = Game::Ff2Pr;
    const CAPABILITIES: &'static [Capability] = &ff1::CAPABILITIES;
    const DOCUMENT: &'static str = "Ff2StandDocument";

    fn new(document: Document, tables: &'static GameTables) -> Self {
        Self {
            inner: Ff1Model::new(document, tables),
            tables,
        }
    }

    fn document(&self) -> &Document {
        Ff1Model::document(&self.inner)
    }

    fn tables(&self) -> &'static GameTables {
        self.tables
    }

    fn rows(&mut self, collection: &FieldId) -> Result<Vec<u32>, EditError> {
        Ff1Model::rows(&mut self.inner, collection)
    }

    fn get(&mut self, field: &FieldId) -> Result<Value, EditError> {
        Ff1Model::get(&mut self.inner, field)
    }

    fn field_meta(&self, field: &FieldId) -> Option<FieldMeta> {
        Ff1Model::field_meta(&self.inner, field)
    }

    fn collection_meta(&self, collection: &FieldId) -> Option<CollectionMeta> {
        Ff1Model::collection_meta(&self.inner, collection)
    }

    fn summary(&mut self) -> Result<SaveSummary, FormatError> {
        Ff1Model::summary(&mut self.inner)
    }

    fn view(&mut self, language: Language) -> Result<SaveView, FormatError> {
        Ff1Model::view(&mut self.inner, language)
    }

    fn apply(&mut self, edit: &Edit, policy: CapPolicy) -> Result<Edit, EditError> {
        Ff1Model::apply(&mut self.inner, edit, policy)
    }

    fn apply_game(&mut self, edit: &Edit) -> Result<GameChange, EditError> {
        Ff1Model::apply_game(&mut self.inner, edit)
    }

    fn row_locked(&mut self, row: &RowId) -> Result<bool, EditError> {
        Ff1Model::row_locked(&mut self.inner, row)
    }

    fn field_locked(&self, field: &FieldId) -> Result<bool, EditError> {
        Ff1Model::field_locked(&self.inner, field)
    }
}

/// FF I and the FF II stand-in, both real `SlotFormat`s.
pub(crate) fn ff2_registry() -> Registry {
    Registry::from_formats(vec![
        Arc::new(Ff1Format::with_language(Language::En)),
        Arc::new(SlotFormat::<Ff2Stand>::with_language(Language::En)),
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
