//! The party overview: one card per character, read from the view by field id.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::sync::Arc;

use common::{Editing, Part, editing, editing_site, field, items, open_editor, pages};
use neoffse::core::{Capability, Entity, FieldId, FieldMeta, FieldView, Game, SaveView, Section, Shape, TableKind, Value};
use neoffse::data::{Language, tables_for};
use neoffse::pr::ff1::names;
use neoffse_app::editor::{Input, overview};
use neoffse_app::state::AppState;
use neoffse_app::{CharacterCard, EditorViewModel, OriginalSprites};

const NAMES: [&str; 4] = ["Aria", "Bram", "Cleo", "Zoé"];

fn cards(ed: &Editing) -> Vec<CharacterCard> {
    items(&ed.editor().overview().cards)
}

fn charges(card: &CharacterCard) -> Vec<String> {
    items(&card.charges).iter().map(ToString::to_string).collect()
}

fn text_of(ed: &Editing, page: &str, label: &str) -> String {
    field(&ed.data(), Part::Party, page, label).text.to_string()
}

fn key_of(ed: &Editing, page: &str, label: &str) -> i32 {
    field(&ed.data(), Part::Party, page, label).key
}

fn character(id: u32) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id)
}

fn int_field(id: &FieldId, name: &'static str, label: &str, value: Value) -> FieldView {
    FieldView::new(id.clone().with_name(name), FieldMeta::new(label, Shape::Int { min: 0, max: 999 }), value)
}

fn party_of(entity: Entity) -> SaveView {
    SaveView::new(vec![Section::new(Capability::Party, vec![entity])])
}

#[test]
fn cards_follow_the_party_pages() {
    let site = editing_site();
    let ed = editing(&site);
    let cards = cards(&ed);
    let data = ed.data();
    let page_labels: Vec<String> = pages(&data.party).iter().map(|page| page.label.to_string()).collect();

    assert_eq!(cards.iter().map(|card| card.name.to_string()).collect::<Vec<_>>(), NAMES);
    for (index, card) in cards.iter().enumerate() {
        assert_eq!(usize::try_from(card.page).unwrap(), index);
        assert_eq!(page_labels[usize::try_from(card.page).unwrap()], card.name.as_str());
    }
}

#[test]
fn a_card_reads_the_values_of_its_page() {
    let site = editing_site();
    let ed = editing(&site);
    let cards = cards(&ed);
    for card in &cards {
        let name = card.name.as_str();
        assert_eq!(card.hp, text_of(&ed, name, "HP").as_str());
        assert_eq!(card.level, text_of(&ed, name, "Level").as_str());
        assert_eq!(card.exp, text_of(&ed, name, "Experience").as_str());
        assert_eq!(card.job, field(&ed.data(), Part::Party, name, "Job").shown);
        let expected: Vec<String> = (1..=8).map(|level| text_of(&ed, name, &format!("Charges, level {level}"))).collect();
        assert_eq!(charges(card), expected, "charges of {name}");
    }
    assert_eq!(cards[0].exp, "100100");
    assert_eq!(charges(&cards[0])[0], "2");
}

#[test]
fn the_spell_levels_come_in_order() {
    let site = editing_site();
    let ed = editing(&site);
    let cards = cards(&ed);
    let cleo = &cards[2];
    assert_eq!(cleo.name, "Cleo");
    let charges = charges(cleo);
    assert_eq!(charges.len(), 8);
    for (k, value) in charges.iter().enumerate() {
        assert_eq!(*value, ((k + 1 + 3) % 10).to_string());
    }
}

#[test]
fn the_initial_is_the_first_letter() {
    let site = editing_site();
    let ed = editing(&site);
    let cards = cards(&ed);
    assert_eq!(cards[3].initial, "Z");

    let key = key_of(&ed, "Aria", "Name");
    assert!(ed.editor().commit(key, Input::Text("zed".to_owned())).is_none());
    let renamed = &self::cards(&ed)[0];
    assert_eq!(renamed.name, "zed");
    assert_eq!(renamed.initial, "Z");

    let unnamed = Entity::new(character(7), "Character 7", Vec::new());
    let built = overview(&party_of(unnamed), Game::Ff1Pr, Language::En, &OriginalSprites);
    assert_eq!(items(&built.cards)[0].initial, "C");
    let empty = Entity::new(character(8), "", Vec::new());
    let built = overview(&party_of(empty), Game::Ff1Pr, Language::En, &OriginalSprites);
    assert_eq!(items(&built.cards)[0].initial, "");
}

#[test]
fn the_gil_comes_from_the_inventory() {
    let site = editing_site();
    let ed = editing(&site);
    assert_eq!(ed.editor().overview().gil, "123456");

    let key = field(&ed.data(), Part::Inventory, "Gil", "Gil").key;
    assert!(ed.editor().commit(key, Input::Text("500".to_owned())).is_none());
    assert_eq!(ed.editor().overview().gil, "500");
}

#[test]
fn a_commit_refreshes_the_overview_in_the_bridge() {
    let site = editing_site();
    let ui = open_editor(&site);
    assert!(ui.bridge().get_overview().present);
    let data = ui.bridge_data();
    let hp = field(&data, Part::Party, "Aria", "HP").key;
    let exp = field(&data, Part::Party, "Aria", "Experience").key;

    ui.commit(hp, "77");
    ui.commit(exp, "4242");
    let overview = ui.bridge().get_overview();
    let first = items(&overview.cards).into_iter().next().unwrap();
    assert!(overview.present);
    assert_eq!(first.hp, "77");
    assert_eq!(first.exp, "4242");
    assert_eq!(overview.gil, "123456");
}

#[test]
fn a_game_without_a_builder_gets_an_empty_overview() {
    let site = editing_site();
    let ed = editing(&site);
    let view = ed.with_session(|session| session.view().unwrap());
    let built = overview(&view, Game::Ff2Pr, Language::En, &OriginalSprites);
    assert!(!built.present);
    assert_eq!(items(&built.cards).len(), 0);
    assert_eq!(built.gil, "");
}

#[test]
fn missing_fields_give_empty_strings() {
    let only_name = FieldView::new(
        character(1).with_name(names::NAME),
        FieldMeta::new("Name", Shape::Text),
        Value::Text("Hero".to_owned()),
    );
    let view = SaveView::new(vec![
        Section::new(Capability::Party, vec![Entity::new(character(1), "Hero", vec![only_name])]),
        Section::new(Capability::Inventory, vec![Entity::new(FieldId::new("elsewhere"), "Elsewhere", Vec::new())]),
    ]);
    let built = overview(&view, Game::Ff1Pr, Language::En, &OriginalSprites);
    let card = &items(&built.cards)[0];
    assert_eq!(items(&built.cards).len(), 1);
    assert_eq!((card.hp.as_str(), card.level.as_str(), card.exp.as_str(), card.job.as_str()), ("", "", "", ""));
    assert_eq!(charges(card), vec![String::new(); 8]);
    assert!(built.present);
    assert_eq!(built.gil, "");
}

#[test]
fn a_wrong_kind_value_gives_an_empty_string() {
    let id = character(1);
    let hp = FieldView::new(
        id.clone().with_name(names::HP),
        FieldMeta::new("HP", Shape::Text),
        Value::Text("many".to_owned()),
    );
    let level = int_field(&id, names::LEVEL, "Level", Value::Int(5));
    let built = overview(&party_of(Entity::new(id, "Hero", vec![hp, level])), Game::Ff1Pr, Language::En, &OriginalSprites);
    let card = &items(&built.cards)[0];
    assert_eq!(card.hp, "");
    assert_eq!(card.level, "5");
}

#[test]
fn an_unknown_job_gives_no_name() {
    let site = editing_site();
    let ed = editing(&site);
    assert_ne!(cards(&ed)[0].job, "");

    ed.editor().set_expert(true);
    let key = key_of(&ed, "Aria", "Job");
    assert!(ed.editor().commit(key, Input::Pick(13)).is_none());
    assert_eq!(cards(&ed)[0].job, "");
}

#[test]
fn the_overview_follows_the_language() {
    let site = editing_site();
    let ed = editing(&site);
    let tables = tables_for(Game::Ff1Pr).unwrap();
    let id = ed.with_session(|session| {
        let view = session.view().unwrap();
        let (_, job) = view.find(&character(1).with_name(names::JOB)).unwrap();
        match job.value {
            Value::Int(id) => u32::try_from(id).unwrap(),
            _ => panic!("the job is an integer"),
        }
    });
    let entry = tables.entry(TableKind::Job, id).unwrap();
    assert_ne!(entry.name(Language::En), entry.name(Language::Fr), "the job names differ between languages");

    let french = EditorViewModel::new(Arc::clone(&ed.flow.state), Language::Fr);
    let card = items(&french.overview().cards).into_iter().next().unwrap();
    assert_eq!(card.job, entry.name(Language::Fr));
    assert_eq!(cards(&ed)[0].job, entry.name(Language::En));
}

#[test]
fn a_failed_view_gives_the_default() {
    let editor = EditorViewModel::new(Arc::new(AppState::new()), Language::En);
    let built = editor.overview();
    assert!(!built.present);
    assert_eq!(items(&built.cards).len(), 0);
    assert_eq!(built.gil, "");
}
