//! Builds a synthetic FF I slot from Rust data, in the game's style: compact,
//! raw UTF-8, string documents nested as the game nests them
//! (`docs/formats/pixel-remaster.md#documents-inside-strings`).
//!
//! Every value is invented and obeys the game's rules: each equipped id sits
//! in the inventory with a count at least its wearers, each slot `count`
//! equals the inventory count of its id, the five placeholders open the
//! inventory, key items are ids of 45 to 61 with `count` 1. The public
//! fields let a test twist one value; numbers a test breaks are raw text.

use neoffse_core::FieldId;
use neoffse_format_pr::document::{Document, Segment};
use neoffse_format_pr::ff1::{Ff1Model, GAME, names};

/// One entry of `equipmentList`.
#[derive(Debug, Clone)]
pub(crate) struct Equip {
    pub key: u32,
    /// Raw number text.
    pub content_id: String,
    /// Raw number text; `None` leaves the key out.
    pub count: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Character {
    pub id: u32,
    /// Raw number text of `characterStatusId`; `None` leaves the key out.
    pub status_id: Option<String>,
    pub job: u32,
    pub name: String,
    /// Raw number text of `addtionalLevel`.
    pub level: String,
    pub exp: u32,
    pub hp: u32,
    pub max_hp_gain: u32,
    /// Power, vitality, agility, intelligence, spirit, luck, magic.
    pub stats: [u32; 7],
    /// Entries of `currentMpCountList` in stored order: spell level, charges.
    pub charges: Vec<(u32, u32)>,
    /// Entries of `addtionalMaxMpCountList` in stored order.
    pub charges_gain: Vec<(u32, u32)>,
    /// Entries of `equipmentList` in stored order.
    pub equipment: Vec<Equip>,
    /// Ability objects of `abilityList`: `abilityId`, `contentId`.
    pub abilities: Vec<(u32, u32)>,
    /// A key of the character or of its `parameter` to leave out, by name.
    pub omit: Option<&'static str>,
}

/// A row of an item list: the JSON text of the element, so a test can break it.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub text: String,
}

impl Row {
    /// The canonical element text of the game.
    pub(crate) fn new(content_id: u32, count: u32) -> Self {
        Self::raw(format!("{{\"contentId\":{content_id},\"count\":{count}}}"))
    }

    pub(crate) fn raw(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SlotData {
    /// A space after each top-level colon, so a rewritten level shows.
    pub spaced: bool,
    /// `false` leaves `userData` out.
    pub with_user_data: bool,
    /// Raw number text of `owendGil`; `None` leaves the key out.
    pub gil: Option<String>,
    /// Raw number text of `userData.playTime`.
    pub play_time: String,
    pub characters: Vec<Character>,
    /// Replaces the document of `ownedCharacterList` as given.
    pub character_list: Option<String>,
    pub inventory: Vec<Row>,
    pub key_items: Vec<Row>,
    pub sort_ids: Vec<u32>,
}

/// The inventory of the standard slot: content id, count, in document order.
pub(crate) const INVENTORY: [(u32, u32); 29] = [
    (62, 5),
    (130, 9),
    (131, 5),
    (132, 9),
    (133, 9),
    (2, 7),
    (3, 4),
    (5, 2),
    (10, 3),
    (17, 1),
    (18, 2),
    (89, 1),
    (92, 1),
    (116, 1),
    (67, 1),
    (170, 1),
    (173, 1),
    (166, 1),
    (154, 1),
    (158, 1),
    (160, 1),
    (151, 1),
    (183, 2),
    (187, 1),
    (188, 1),
    (202, 4),
    (100, 1),
    (103, 1),
    (165, 1),
];

/// Key items of the standard slot, ids of 45 to 61.
pub(crate) const KEY_ITEMS: [u32; 9] = [45, 46, 48, 49, 50, 51, 52, 53, 61];

const NAMES: [&str; 4] = ["Aria", "Bram", "Cleo", "Zoé"];
const STATUS: [u32; 4] = [1, 2, 5, 6];
const JOBS: [u32; 4] = [7, 8, 11, 12];
const EQUIPPED: [[u32; 5]; 4] = [
    [89, 130, 154, 183, 202],
    [92, 170, 158, 187, 202],
    [116, 173, 160, 188, 202],
    [67, 166, 151, 183, 202],
];

fn count_of(id: u32) -> u32 {
    INVENTORY.iter().find(|(content, _)| *content == id).map_or(0, |(_, count)| *count)
}

fn standard_character(index: usize) -> Character {
    let id = u32::try_from(index).unwrap() + 1;
    let base = id;
    Character {
        id,
        status_id: Some(STATUS[index].to_string()),
        job: JOBS[index],
        name: NAMES[index].to_owned(),
        level: (30 + id).to_string(),
        exp: id * 100_100,
        hp: 300 + id * 11,
        max_hp_gain: 400 + id * 11,
        stats: [10, 20, 30, 40, 50, 60, 70].map(|stat| stat + base),
        charges: (1..=8).map(|level| (level, (level + id) % 10)).collect(),
        charges_gain: (1..=8).map(|level| (level, (2 * level + id) % 8)).collect(),
        equipment: (1..=5)
            .zip(EQUIPPED[index])
            .map(|(key, content)| Equip {
                key,
                content_id: content.to_string(),
                count: Some(count_of(content).to_string()),
            })
            .collect(),
        abilities: if id == 3 { vec![(5, 213), (6, 214)] } else { Vec::new() },
        omit: None,
    }
}

impl SlotData {
    /// The standard slot: four characters, the rules of the module docs.
    pub(crate) fn standard() -> Self {
        Self {
            spaced: false,
            with_user_data: true,
            gil: Some("123456".to_owned()),
            play_time: "13621.814333377138".to_owned(),
            characters: (0..4).map(standard_character).collect(),
            character_list: None,
            inventory: INVENTORY.iter().map(|(id, count)| Row::new(*id, *count)).collect(),
            key_items: KEY_ITEMS.iter().map(|id| Row::new(*id, 1)).collect(),
            sort_ids: Vec::new(),
        }
    }

    /// The slot as JSON text.
    pub(crate) fn render(&self) -> String {
        super::ff1_render::slot(self)
    }
}

/// A model over `text`, with the FF I tables.
pub(crate) fn model(text: &str) -> Ff1Model {
    let document = Document::parse(text).unwrap();
    Ff1Model::new(document, neoffse_data::tables_for(GAME).unwrap())
}

/// A model whose inventory ends with the canonical row `(canonical_key, 3)`
/// and then the canonical row `(escaped_key, 1)` stored with `\u0022`
/// escapes for its quotes: the decoded text is canonical, the stored token is
/// not what the model writes. Returns the model and the slot text.
pub(crate) fn model_with_escaped_row(canonical_key: u32, escaped_key: u32) -> (Ff1Model, String) {
    let mut data = SlotData::standard();
    data.inventory.push(Row::new(canonical_key, 3));
    let row = Row::new(escaped_key, 1);
    data.inventory.push(row.clone());
    let mut document = Document::parse(&data.render()).unwrap();
    let path = [
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("normalOwnedItemList"),
        Segment::Doc,
        Segment::Key("target"),
        Segment::Index(data.inventory.len() - 1),
    ];
    let token = super::embed_with(&row.text, true);
    document.set_raw(&path, &token).unwrap();
    assert_eq!(document.raw(&path).unwrap(), token);
    assert!(token.contains(&format!("{}u0022", char::from(92))), "{token}");
    let text = document.to_json();
    (Ff1Model::new(document, neoffse_data::tables_for(GAME).unwrap()), text)
}

/// A model over the standard slot.
pub(crate) fn standard_model() -> Ff1Model {
    model(&SlotData::standard().render())
}

/// The id of a field of a character.
pub(crate) fn character_field(id: u32, name: &'static str) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id).with_name(name)
}

/// The keys of the JSON object `text`, in order, repeats included.
pub(crate) fn keys(text: &str) -> Vec<String> {
    use serde::de::{Deserializer as _, IgnoredAny, MapAccess, Visitor};

    struct Keys;
    impl<'de> Visitor<'de> for Keys {
        type Value = Vec<String>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("an object")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
            let mut keys = Vec::new();
            while let Some((key, IgnoredAny)) = map.next_entry::<String, IgnoredAny>()? {
                keys.push(key);
            }
            Ok(keys)
        }
    }
    serde_json::Deserializer::from_str(text).deserialize_map(Keys).unwrap()
}
