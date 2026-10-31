//! The FF II party overview: one card per character with name, HP and MP and
//! no level row data. FF I keeps its card.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Editing, Part, editing, editing_site, ff2_editing, ff2_site, field, items};
use neoffse::core::{Capability, Entity, FieldId, FieldMeta, FieldView, Game, SaveView, Section, Shape, Value};
use neoffse::data::Language;
use neoffse::pr::ff2::names;
use neoffse_app::editor::{Input, overview};
use neoffse_app::{CharacterCard, OriginalSprites, ProviderChain};

const NAMES: [&str; 4] = ["Ivo", "Mara", "Tane", "Élan"];

fn cards(ed: &Editing) -> Vec<CharacterCard> {
    items(&ed.editor().overview().cards)
}

#[test]
fn the_overview_is_present_with_one_card_per_character() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let overview = ed.editor().overview();
    let cards = items(&overview.cards);
    assert!(overview.present);
    assert_eq!(cards.iter().map(|card| card.name.to_string()).collect::<Vec<_>>(), NAMES);
    for (index, card) in cards.iter().enumerate() {
        assert_eq!(usize::try_from(card.page).unwrap(), index);
    }
    assert_eq!(items(&overview.card_of_page), [0, 1, 2, 3]);
}

#[test]
fn a_card_holds_hp_and_mp_with_their_maximum() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let cards = cards(&ed);
    let shown: Vec<(String, String, String, String)> = cards
        .iter()
        .map(|card| (card.hp.to_string(), card.hp_max.to_string(), card.mp.to_string(), card.mp_max.to_string()))
        .collect();
    // The party page shows the same numbers: the card reads the view, not a copy.
    let data = ed.data();
    for (card, name) in cards.iter().zip(NAMES) {
        let page = |label: &str| field(&data, Part::Party, name, label).text.to_string();
        assert_eq!(card.hp, page("HP").as_str());
        assert_eq!(card.mp, page("MP").as_str());
        assert_eq!(card.hp_max, page("Max HP").as_str());
        assert_eq!(card.mp_max, page("Max MP").as_str());
    }
    assert_eq!(shown[0], ("140".into(), "150".into(), "20".into(), "24".into()));
    assert_eq!(shown[3], ("1500".into(), "1575".into(), "150".into(), "183".into()));
}

#[test]
fn a_card_has_no_level_experience_job_or_charges() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    for card in cards(&ed) {
        assert!(!card.has_level, "{}", card.name);
        assert_eq!(
            (card.level.as_str(), card.exp.as_str(), card.next_exp.as_str(), card.job.as_str()),
            ("", "", "", ""),
            "{}",
            card.name
        );
        assert_eq!(items(&card.charges).len(), 0, "{}", card.name);
    }
}

#[test]
fn the_overview_carries_the_gil() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    assert_eq!(ed.editor().overview().gil, "1234567");
}

#[test]
fn editing_hp_updates_the_card() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let key = field(&ed.data(), Part::Party, "Ivo", "HP").key;
    assert!(ed.editor().commit(key, Input::Text("77".to_owned())).is_none());
    let card = &cards(&ed)[0];
    assert_eq!((card.hp.as_str(), card.hp_max.as_str()), ("77", "150"));
    let key = field(&ed.data(), Part::Party, "Ivo", "MP").key;
    assert!(ed.editor().commit(key, Input::Text("9".to_owned())).is_none());
    assert_eq!(cards(&ed)[0].mp, "9");
}

#[test]
fn the_portrait_stays_empty_until_the_provider_has_one() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    ed.editor().set_assets(std::sync::Arc::new(ProviderChain::new(Vec::new())));
    for card in cards(&ed) {
        assert_eq!(card.portrait.size().width, 0, "{}", card.name);
        assert!(!card.initial.is_empty(), "{}", card.name);
    }
}

#[test]
fn the_ff1_card_keeps_its_level_and_has_no_mp() {
    let site = editing_site();
    let ed = editing(&site);
    for card in cards(&ed) {
        assert!(card.has_level, "{}", card.name);
        assert_eq!((card.mp.as_str(), card.mp_max.as_str()), ("", ""), "{}", card.name);
        assert_eq!(items(&card.charges).len(), 8, "{}", card.name);
        assert_ne!(card.level, "", "{}", card.name);
    }
}

#[test]
fn a_missing_mp_field_gives_empty_strings() {
    let character = FieldId::new(names::CHARACTER).with_id(1);
    let hp = FieldView::new(
        character.clone().with_name(names::HP),
        FieldMeta::new("HP", Shape::Int { min: 0, max: 9999 }),
        Value::Int(12),
    );
    let view = SaveView::new(vec![Section::new(Capability::Party, vec![Entity::new(character, "Hero", vec![hp])])]);
    let built = overview(&view, Game::Ff2Pr, Language::En, &OriginalSprites);
    let card = &items(&built.cards)[0];
    assert!(built.present);
    assert_eq!(
        (card.hp.as_str(), card.mp.as_str(), card.mp_max.as_str(), card.hp_max.as_str()),
        ("12", "", "", "")
    );
    assert_eq!(built.gil, "");
}
