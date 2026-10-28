//! The committed FF I slot fixture, its generator and the game rules it obeys.
//!
//! The JSON is synthetic. Regenerate the file with
//! `NEOFFSE_WRITE_FIXTURES=1 cargo test -p neoffse-format-pr --test ff1_fixture write_fixture`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_path;
use common::ff1_rules::assert_game_rules;
use common::ff1_save::{SlotData, keys, model};
use neoffse_core::{FieldId, Value};
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff1::names;

const FILE: &str = "pr/ff1-slot.json";

#[test]
fn the_committed_slot_matches_the_generator() {
    let committed = neoffse_testkit::read_fixture(FILE).unwrap();
    assert!(
        committed == SlotData::standard().render().into_bytes(),
        "{FILE} differs from the generator: regenerate it"
    );
}

#[test]
fn write_fixture() {
    if std::env::var_os("NEOFFSE_WRITE_FIXTURES").is_none() {
        return;
    }
    let path = neoffse_testkit::fixture_path(FILE);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, SlotData::standard().render()).unwrap();
}

fn unique(keys: &[String]) -> bool {
    keys.iter().collect::<std::collections::HashSet<_>>().len() == keys.len()
}

fn text_at(document: &mut Document, path: &[Segment<'_>]) -> String {
    let Scalar::Text(text) = document.get(path).unwrap() else {
        panic!("no string at the path")
    };
    text
}

fn slot_field(id: u32, slot: u32) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id).with_name(names::EQUIPMENT).with_id(slot)
}

#[test]
fn the_fixture_is_game_style_and_covers_the_cases() {
    let text = SlotData::standard().render();
    let mut document = Document::parse(&text).unwrap();
    assert_eq!(document.to_json(), text);
    // `docs/formats/pixel-remaster.md#slot-keys`
    let order = [
        "id",
        "pictureData",
        "userData",
        "configData",
        "dataStorage",
        "mapData",
        "timeStamp",
        "playTime",
        "clearFlag",
        "isCompleteFlag",
    ];
    assert_eq!(keys(&text), order);
    let user = text_at(&mut document, &[Segment::Key("userData")]);
    assert!(unique(&keys(&user)));
    let first = character_path(0, &[]);
    assert!(unique(&keys(&text_at(&mut document, &first))));
    let play_time = [Segment::Key("userData"), Segment::Doc, Segment::Key("playTime")];
    let raw = document.raw(&play_time).unwrap().into_owned();
    assert_eq!(raw, "13621.814333377138");
    assert_eq!(raw.chars().filter(char::is_ascii_digit).count(), 17);
    assert_eq!(document.raw(&[Segment::Key("playTime")]).unwrap(), "0.0");

    let mut model = model(&text);
    let name = model.get(&FieldId::new(names::CHARACTER).with_id(4).with_name(names::NAME)).unwrap();
    assert_eq!(name, Value::Text("Zoé".to_owned()));
    assert!(!"Zoé".is_ascii());
    assert_eq!(model.get(&slot_field(4, 1)).unwrap(), Value::Int(67));
    assert_eq!(model.get(&slot_field(1, 2)).unwrap(), Value::Int(130));
    let inventory = model.rows(&FieldId::new(names::INVENTORY)).unwrap();
    assert_eq!(inventory[..5], [62, 130, 131, 132, 133]);
    assert_eq!(model.rows(&FieldId::new(names::KEY_ITEMS)).unwrap().len(), 9);
}

#[test]
fn the_fixture_obeys_the_game_rules() {
    let mut model = model(&SlotData::standard().render());
    assert_game_rules(&mut model, true);
    let inventory = model.rows(&FieldId::new(names::INVENTORY)).unwrap();
    assert_eq!(inventory[..5], [62, 130, 131, 132, 133]);
    let key_items = FieldId::new(names::KEY_ITEMS);
    for key in model.rows(&key_items).unwrap() {
        assert!((45..=61).contains(&key), "key item {key} outside 45 to 61");
        let count = model.get(&key_items.clone().with_id(key).with_name(names::COUNT)).unwrap();
        assert_eq!(count, Value::Int(1));
    }
    let mut worn = Vec::new();
    for id in 1..=4 {
        for slot in 1..=names::EQUIPMENT_SLOTS {
            worn.push(model.get(&slot_field(id, slot)).unwrap());
        }
    }
    for unworn in [100_u32, 103, 165] {
        assert!(!worn.contains(&Value::Int(i64::from(unworn))), "{unworn} is worn");
        assert!(inventory.contains(&unworn), "{unworn} is not in the inventory");
    }
}
