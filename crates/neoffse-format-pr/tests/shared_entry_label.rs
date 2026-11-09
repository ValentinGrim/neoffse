//! The inventory label of a row the tables list without a name is `Entry <id>`; a named row keeps its
//! name and an unlisted key keeps `Item <key>` (`docs/formats/pixel-remaster.md#rows-without-a-name`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::SlotData;
use common::slot_render::Row;
use neoffse_core::{Capability, FieldId, Game, SaveView};
use neoffse_data::{GameTables, Language, Sources};
use neoffse_format_pr::document::Document;
use neoffse_format_pr::ff1::{Ff1Model, names};

const SOURCES: Sources<'static> = Sources {
    content: r#"
[[content]]
id = 5
kind = "item"
name_en = "Ether"
name_fr = "Éther"

[[content]]
id = 6
kind = "item"

[[content]]
id = 7
kind = "item"
name_en = ""
name_fr = ""

[[content]]
id = 46
kind = "key_item"
"#,
    abilities: r#"
[[ability]]
id = 1
content_id = 209
type_id = 4
kind = "command"
level = 0
name_en = "Attack"
name_fr = "Attaque"
"#,
    jobs: "[[job]]\nid = 1\nname_en = \"Warrior\"\nname_fr = \"Guerrier\"\n",
    commands: "[[command]]\nid = 1\nname_en = \"Attack\"\nname_fr = \"Attaque\"\n",
    monsters: None,
    bounds: "[bounds]\nMAX_GIL = 999999\n",
    growth: None,
    skills: None,
};

fn view_over_small_tables() -> SaveView {
    let tables = GameTables::parse(Game::Ff1Pr, &SOURCES).unwrap();
    // The model keeps the tables for its whole life: leak them for `'static`.
    let tables: &'static GameTables = Box::leak(Box::new(tables));
    let mut slot = SlotData::standard();
    slot.inventory = vec![Row::new(5, 2), Row::new(6, 1), Row::new(7, 1), Row::new(999, 1)];
    slot.key_items = vec![Row::new(46, 1), Row::new(998, 1)];
    let mut model = Ff1Model::new(Document::parse(&slot.render()).unwrap(), tables);
    model.view(Language::En).unwrap()
}

fn label(view: &SaveView, list: &'static str, key: u32) -> String {
    let id = FieldId::new(list).with_id(key);
    let section = view.section(Capability::Inventory).unwrap();
    section.entities.iter().find(|entity| entity.id == id).unwrap().label.clone()
}

#[test]
fn a_listed_row_without_a_name_reads_entry() {
    let view = view_over_small_tables();
    assert_eq!(label(&view, names::INVENTORY, 6), "Entry 6");
    assert_eq!(label(&view, names::KEY_ITEMS, 46), "Entry 46");
}

#[test]
fn an_empty_name_reads_entry() {
    let view = view_over_small_tables();
    assert_eq!(label(&view, names::INVENTORY, 7), "Entry 7");
}

#[test]
fn a_named_row_keeps_its_name() {
    let view = view_over_small_tables();
    assert_eq!(label(&view, names::INVENTORY, 5), "Ether");
}

#[test]
fn an_unlisted_key_keeps_its_old_text() {
    let view = view_over_small_tables();
    assert_eq!(label(&view, names::INVENTORY, 999), "Item 999");
    assert_eq!(label(&view, names::KEY_ITEMS, 998), "Key item 998");
}
