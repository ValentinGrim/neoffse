//! The portrait of each FF II card: picked by the job id, which names the
//! person, never by the position. No window shows: the checks run on Slint's
//! testing backend.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::sync::Arc;

use common::{ff2_editing, ff2_site, ff2_ui, items, jobs, party, pixels, resize, size_of};
use i_slint_backend_testing::ElementHandle;
use neoffse::core::{FieldId, Game, SaveView, Value};
use neoffse::data::Language;
use neoffse::pr::ff2::names;
use neoffse_app::assets::person_of;
use neoffse_app::editor::overview;
use neoffse_app::{AssetProvider, CharacterCard, OriginalSprites, ProviderChain, refresh_editor};

const SPRITE: &str = "PortraitFrame::sprite";

fn sprite(job: u32) -> Option<(u32, u32, Vec<u8>)> {
    pixels(&OriginalSprites.portrait(Game::Ff2Pr, job).expect("a sprite"))
}

fn cards_of(view: &SaveView) -> Vec<CharacterCard> {
    items(&overview(view, Game::Ff2Pr, Language::En, &OriginalSprites).cards)
}

#[test]
fn each_card_carries_the_portrait_of_its_job() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let ids: Vec<u32> = ed.with_session(|session| {
        let view = session.view().unwrap();
        (1..=4)
            .map(
                |id| match view.find(&FieldId::new(names::CHARACTER).with_id(id).with_name(names::JOB)).unwrap().1.value {
                    Value::Int(job) => u32::try_from(job).unwrap(),
                    _ => panic!("the job is an integer"),
                },
            )
            .collect()
    });
    let cards = items(&ed.editor().overview().cards);
    assert_eq!(cards.len(), 4);
    for (card, id) in cards.iter().zip(ids) {
        assert!(person_of(Game::Ff2Pr, id).is_some(), "job {id} has a person");
        assert_eq!(size_of(&card.portrait), (32, 32));
        assert_eq!(pixels(&card.portrait), sprite(id), "the portrait of job {id}");
    }
}

#[test]
fn the_portrait_follows_the_job_not_the_position() {
    let view = party(&["Zed", "Ann", "Bob"], &jobs(&[9, 1, 5]));
    let got: Vec<_> = cards_of(&view).iter().map(|card| pixels(&card.portrait)).collect();
    assert_eq!(got, vec![sprite(9), sprite(1), sprite(5)], "Leon, Firion, Josef");
    assert_ne!(sprite(9), sprite(1));
}

#[test]
fn leons_two_statuses_share_one_image() {
    // Status 9 and status 10 both store job 9 (People and party in FF II).
    let view = party(&["Leon", "Leon"], &jobs(&[9, 9]));
    let cards = cards_of(&view);
    assert_eq!(cards.len(), 2);
    assert_eq!(pixels(&cards[0].portrait), sprite(9));
    assert_eq!(pixels(&cards[0].portrait), pixels(&cards[1].portrait));
}

#[test]
fn a_missing_negative_huge_or_unknown_job_keeps_the_initial() {
    let view = party(
        &["Ann", "Bob", "Cat", "Dan", "Eve", "Fay"],
        &[
            None,
            Some(Value::Int(-1)),
            Some(Value::Int(i64::MAX)),
            Some(Value::Text("1".to_owned())),
            Some(Value::Int(0)),
            Some(Value::Int(10)),
        ],
    );
    let cards = cards_of(&view);
    assert_eq!(cards.len(), 6);
    for card in &cards {
        assert_eq!(size_of(&card.portrait), (0, 0), "{}", card.name);
        assert_eq!(card.initial, card.name.chars().next().unwrap().to_string());
    }
}

#[test]
fn set_assets_with_the_empty_chain_empties_every_portrait() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let before = items(&ed.editor().overview().cards);
    assert!(before.iter().all(|card| size_of(&card.portrait) == (32, 32)));

    ed.editor().set_assets(Arc::new(ProviderChain::new(Vec::new())));
    let after = items(&ed.editor().overview().cards);
    assert_eq!(after.len(), 4);
    assert!(after.iter().all(|card| size_of(&card.portrait) == (0, 0)));
    assert!(after.iter().all(|card| !card.initial.is_empty()));
}

#[test]
fn the_cards_draw_a_sprite_each_until_the_chain_empties() {
    let site = ff2_site();
    let ui = ff2_ui(&site);
    resize(&ui, 1280.0, 800.0);
    assert_eq!(ElementHandle::find_by_element_id(&ui.window, SPRITE).count(), 4);

    ui.app.editor.set_assets(Arc::new(ProviderChain::new(Vec::new())));
    refresh_editor(&ui.window, &ui.app);
    assert_eq!(ElementHandle::find_by_element_id(&ui.window, SPRITE).count(), 0);
}
