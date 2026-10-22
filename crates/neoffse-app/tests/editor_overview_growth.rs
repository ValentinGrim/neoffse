//! The computed fields on the cards and the character page: the maximum HP
//! and the experience to the next level, read from the view by field id
//! (`docs/formats/pixel-remaster.md#maximum-hp-and-experience-tables`).

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Editing, Part, editing, editing_site, field, fields, items, open_editor, page};
use neoffse::core::{Capability, Entity, FieldId, FieldMeta, FieldView, Game, SaveView, Section, Shape, Value};
use neoffse::data::Language;
use neoffse::pr::ff1::names;
use neoffse_app::editor::{Input, overview};
use neoffse_app::{CharacterCard, FieldKind};

const NAMES: [&str; 4] = ["Aria", "Bram", "Cleo", "Zoé"];

fn cards(ed: &Editing) -> Vec<CharacterCard> {
    items(&ed.editor().overview().cards)
}

fn key_of(ed: &Editing, page: &str, label: &str) -> i32 {
    field(&ed.data(), Part::Party, page, label).key
}

fn text_of(ed: &Editing, page: &str, label: &str) -> String {
    field(&ed.data(), Part::Party, page, label).text.to_string()
}

fn labels(ed: &Editing, page_label: &str) -> Vec<String> {
    fields(&page(&ed.data(), Part::Party, page_label))
        .iter()
        .map(|row| row.label.to_string())
        .collect()
}

fn commit(ed: &Editing, page: &str, label: &str, text: &str) {
    let key = key_of(ed, page, label);
    assert!(ed.editor().commit(key, Input::Text(text.to_owned())).is_none(), "commit of {label} on {page}");
}

#[test]
fn cards_read_the_computed_fields() {
    let site = editing_site();
    let ed = editing(&site);
    let cards = cards(&ed);
    let max_hp = ["446", "452", "466", "469"];
    let next = ["3711", "7099", "18425", "5323"];
    for (index, name) in NAMES.iter().enumerate() {
        assert_eq!(cards[index].hp_max, max_hp[index], "hp-max of {name}");
        assert_eq!(cards[index].next_exp, next[index], "next-exp of {name}");
        assert_eq!(cards[index].hp_max, text_of(&ed, name, "Max HP").as_str(), "page Max HP of {name}");
        assert_eq!(cards[index].next_exp, text_of(&ed, name, "Next level").as_str(), "page Next level of {name}");
    }
}

#[test]
fn the_computed_rows_are_read_only_labels() {
    let site = editing_site();
    let ed = editing(&site);
    for expert in [false, true] {
        ed.editor().set_expert(expert);
        for label in ["Max HP", "Next level"] {
            let row = field(&ed.data(), Part::Party, "Aria", label);
            assert_eq!(row.kind, FieldKind::Label, "{label}, expert {expert}");
            assert!(!row.editable, "{label} editable, expert {expert}");
            assert!(row.locked, "{label} unlocked, expert {expert}");
        }
    }
}

#[test]
fn the_computed_rows_follow_their_sources_on_the_page() {
    let site = editing_site();
    let ed = editing(&site);
    let labels = labels(&ed, "Aria");
    assert_eq!(labels.len(), 43);
    assert_eq!(labels[..8], ["Name", "Job", "Level", "Experience", "Next level", "HP", "Max HP", "Max HP gain"]);
    assert_eq!(labels[8], "Power");
}

#[test]
fn a_commit_moves_the_card_values() {
    let site = editing_site();
    let ed = editing(&site);
    commit(&ed, "Bram", "Experience", "100000");
    let bram = &cards(&ed)[1];
    assert_eq!(bram.next_exp, "3811");
    assert_eq!(bram.level, "31");

    commit(&ed, "Aria", "Max HP gain", "1000");
    assert_eq!(cards(&ed)[0].hp_max, "999");
    commit(&ed, "Aria", "Max HP gain", "100");
    assert_eq!(cards(&ed)[0].hp_max, "135");
}

#[test]
fn a_commit_refreshes_the_bridge_cards() {
    let site = editing_site();
    let ui = open_editor(&site);
    let first = |ui: &common::Ui| items(&ui.bridge().get_overview().cards).into_iter().next().unwrap();

    let key = field(&ui.bridge_data(), Part::Party, "Bram", "Experience").key;
    ui.commit(key, "100000");
    let overview = ui.bridge().get_overview();
    let bram = items(&overview.cards).into_iter().nth(1).unwrap();
    assert_eq!(bram.next_exp, "3811");
    assert_eq!(bram.level, "31");

    let key = field(&ui.bridge_data(), Part::Party, "Aria", "Max HP gain").key;
    ui.commit(key, "1000");
    assert_eq!(first(&ui).hp_max, "999");
    let key = field(&ui.bridge_data(), Part::Party, "Aria", "Max HP gain").key;
    ui.commit(key, "100");
    assert_eq!(first(&ui).hp_max, "135");
}

#[test]
fn level_99_empties_next_exp() {
    let site = editing_site();
    let ed = editing(&site);
    commit(&ed, "Aria", "Level", "99");
    let aria = &cards(&ed)[0];
    assert_eq!(aria.level, "99");
    assert_eq!(aria.next_exp, "");
    assert_eq!(aria.hp_max, "446");
    let labels = labels(&ed, "Aria");
    assert_eq!(labels.len(), 42);
    assert!(!labels.iter().any(|label| label == "Next level"));
}

#[test]
fn missing_and_wrong_kind_computed_fields_give_empty_strings() {
    let id = FieldId::new(names::CHARACTER).with_id(1);
    let party_of = |fields: Vec<FieldView>| SaveView::new(vec![Section::new(Capability::Party, vec![Entity::new(id.clone(), "Hero", fields)])]);
    let card_of = |view: &SaveView| items(&overview(view, Game::Ff1Pr, Language::En).cards).into_iter().next().unwrap();

    let bare = card_of(&party_of(Vec::new()));
    assert_eq!((bare.hp_max.as_str(), bare.next_exp.as_str()), ("", ""));

    let text = |name: &'static str, label: &str| FieldView::new(id.clone().with_name(name), FieldMeta::new(label, Shape::Text), Value::Text("many".to_owned()));
    let wrong = card_of(&party_of(vec![text(names::MAX_HP, "Max HP"), text(names::NEXT_LEVEL, "Next level")]));
    assert_eq!((wrong.hp_max.as_str(), wrong.next_exp.as_str()), ("", ""));
}
