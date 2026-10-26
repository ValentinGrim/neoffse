//! The portrait of each party card: picked by job id through the asset
//! provider, drawn in the frame of the cards and of the character page. No
//! window shows: the layout checks run on Slint's testing backend.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::{Editing, Part, Ui, editing, editing_site, field, items, open_editor};
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use neoffse::core::{Capability, Entity, FieldId, FieldMeta, FieldView, Game, SaveView, Section, Shape, Value};
use neoffse::data::Language;
use neoffse::pr::ff1::names;
use neoffse_app::editor::{Input, overview};
use neoffse_app::{AssetProvider, CharacterCard, EditorCommand, EditorViewModel, OriginalSprites, ProviderChain, refresh_editor};
use slint::{ComponentHandle, Image, LogicalSize, Rgba8Pixel, SharedPixelBuffer};

const SPRITE: &str = "PortraitFrame::sprite";

/// A provider that counts its calls and answers with a `side` by `side`
/// image.
struct Fake {
    side: u32,
    calls: AtomicUsize,
}

impl Fake {
    fn new(side: u32) -> Arc<Self> {
        Arc::new(Self {
            side,
            calls: AtomicUsize::new(0),
        })
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl AssetProvider for Fake {
    fn portrait(&self, _: Game, _: u32) -> Option<Image> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Some(Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::new(self.side, self.side)))
    }
}

fn cards(ed: &Editing) -> Vec<CharacterCard> {
    items(&ed.editor().overview().cards)
}

/// The size and the bytes of the pixels, `None` for an empty image.
fn pixels(image: &Image) -> Option<(u32, u32, Vec<u8>)> {
    image.to_rgba8().map(|buffer| (buffer.width(), buffer.height(), buffer.as_bytes().to_vec()))
}

fn sprite(job: u32) -> Option<(u32, u32, Vec<u8>)> {
    pixels(&OriginalSprites.portrait(Game::Ff1Pr, job).expect("a sprite"))
}

fn size_of(image: &Image) -> (u32, u32) {
    let size = image.size();
    (size.width, size.height)
}

fn character(id: u32) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id)
}

fn job_field(id: &FieldId, value: Value) -> FieldView {
    FieldView::new(id.clone().with_name(names::JOB), FieldMeta::new("Job", Shape::Int { min: 0, max: 99 }), value)
}

/// A view of characters named `labels`, character ids from 1, each holding
/// the job in `jobs` at its position, `None` for no job field.
fn party(labels: &[&str], jobs: &[Option<Value>]) -> SaveView {
    let entities = labels
        .iter()
        .zip(jobs)
        .enumerate()
        .map(|(index, (label, job))| {
            let id = character(u32::try_from(index + 1).unwrap());
            let fields = job.iter().map(|value| job_field(&id, value.clone())).collect();
            Entity::new(id, *label, fields)
        })
        .collect();
    SaveView::new(vec![Section::new(Capability::Party, entities)])
}

fn jobs(ids: &[i64]) -> Vec<Option<Value>> {
    ids.iter().map(|id| Some(Value::Int(*id))).collect()
}

/// The portraits of the cards `view` gives, in order.
fn portraits(view: &SaveView, assets: &dyn AssetProvider) -> Vec<CharacterCard> {
    items(&overview(view, Game::Ff1Pr, Language::En, assets).cards)
}

fn job_key(ed: &Editing) -> i32 {
    field(&ed.data(), Part::Party, "Aria", "Job").key
}

/// A window of the shown size, the editor open on the cards.
fn sized_ui(site: &common::Site) -> Ui {
    let ui = open_editor(site);
    ui.window.window().set_size(LogicalSize::new(1280.0, 800.0));
    ui
}

fn sprites(ui: &Ui) -> Vec<ElementHandle> {
    ElementHandle::find_by_element_id(&ui.window, SPRITE).collect()
}

fn texts(ui: &Ui) -> Vec<String> {
    ElementHandle::find_by_element_type_name(&ui.window, "CharacterRow")
        .flat_map(|row| row.query_descendants().match_accessible_role(AccessibleRole::Text).find_all())
        .filter_map(|text| text.accessible_label().map(|label| label.to_string()))
        .collect()
}

#[test]
fn each_card_carries_the_portrait_of_its_job() {
    let site = editing_site();
    let ed = editing(&site);
    let ids: Vec<u32> = ed.with_session(|session| {
        let view = session.view().unwrap();
        (1..=4)
            .map(|id| match view.find(&character(id).with_name(names::JOB)).unwrap().1.value {
                Value::Int(job) => u32::try_from(job).unwrap(),
                _ => panic!("the job is an integer"),
            })
            .collect()
    });
    assert!(ids[0] > 6, "the fixture's first character holds a promoted job");

    let cards = cards(&ed);
    assert_eq!(cards.len(), 4);
    for (card, id) in cards.iter().zip(ids) {
        assert_eq!(size_of(&card.portrait), (32, 32));
        assert_eq!(pixels(&card.portrait), sprite(id), "the portrait of job {id}");
    }
}

#[test]
fn the_portrait_follows_the_job_not_the_position() {
    let view = party(&["Zed", "Ann", "Bob"], &jobs(&[12, 1, 5]));
    let cards = portraits(&view, &OriginalSprites);
    let got: Vec<_> = cards.iter().map(|card| pixels(&card.portrait)).collect();
    assert_eq!(got, vec![sprite(6), sprite(1), sprite(5)], "Black Mage, Warrior, White Mage");
    assert_ne!(sprite(6), sprite(1));
}

#[test]
fn editing_the_job_changes_the_portrait() {
    let site = editing_site();
    let ed = editing(&site);
    ed.editor().set_expert(true);
    assert!(ed.editor().commit(job_key(&ed), Input::Pick(8)).is_none());

    let aria = &cards(&ed)[0];
    assert_eq!(pixels(&aria.portrait), sprite(2), "the Thief sprite");
    assert_eq!(aria.initial, "A");
}

#[test]
fn an_unknown_job_keeps_the_initial() {
    let site = editing_site();
    let ed = editing(&site);
    ed.editor().set_expert(true);
    assert!(ed.editor().commit(job_key(&ed), Input::Pick(13)).is_none());

    let aria = &cards(&ed)[0];
    assert_eq!(size_of(&aria.portrait), (0, 0));
    assert_eq!(aria.initial, "A");
    assert_eq!(aria.job, "");
}

#[test]
fn a_missing_negative_or_huge_job_has_no_portrait() {
    let view = party(
        &["Ann", "Bob", "Cat", "Dan"],
        &[None, Some(Value::Int(-1)), Some(Value::Int(i64::MAX)), Some(Value::Text("1".to_owned()))],
    );
    let cards = portraits(&view, &OriginalSprites);
    assert_eq!(cards.len(), 4);
    for card in &cards {
        assert_eq!(size_of(&card.portrait), (0, 0), "{}", card.name);
    }
}

#[test]
fn another_game_gets_no_cards() {
    let site = editing_site();
    let ed = editing(&site);
    let view = ed.with_session(|session| session.view().unwrap());
    let fake = Fake::new(5);
    let built = overview(&view, Game::Ff2Pr, Language::En, fake.as_ref());
    assert!(!built.present);
    assert_eq!(items(&built.cards).len(), 0);
    assert_eq!(fake.calls(), 0);
}

#[test]
fn set_assets_replaces_the_default() {
    let site = editing_site();
    let ed = editing(&site);
    let clone = ed.editor().clone();

    ed.editor().set_assets(Arc::new(ProviderChain::new(Vec::new())));
    for view_model in [ed.editor(), &clone] {
        let built = items(&view_model.overview().cards);
        assert_eq!(built.len(), 4);
        assert!(built.iter().all(|card| size_of(&card.portrait) == (0, 0)));
    }

    clone.set_assets(Fake::new(5));
    let built = cards(&ed);
    assert!(
        built.iter().all(|card| size_of(&card.portrait) == (5, 5)),
        "the original sees the clone's provider"
    );
}

#[test]
fn the_view_model_stays_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<EditorViewModel>();
}

#[test]
fn the_cards_draw_one_sprite_each() {
    let site = editing_site();
    let ui = sized_ui(&site);
    let found = sprites(&ui);
    assert_eq!(found.len(), 4);
    for element in &found {
        let extent = element.size();
        assert_eq!((extent.width, extent.height), (64.0, 64.0));
    }
    // A sprite replaces the initial: the second to fourth letters are not drawn.
    let texts = texts(&ui);
    for initial in ["B", "C", "Z"] {
        assert!(!texts.iter().any(|text| text == initial), "the initial {initial} stays hidden: {texts:?}");
    }
}

#[test]
fn a_card_without_a_portrait_draws_no_sprite() {
    let site = editing_site();
    let ui = sized_ui(&site);
    ui.app.editor.set_assets(Arc::new(ProviderChain::new(Vec::new())));
    refresh_editor(&ui.window, &ui.app);

    assert!(sprites(&ui).is_empty());
    let texts = texts(&ui);
    for initial in ["A", "B", "C", "Z"] {
        assert!(texts.iter().any(|text| text == initial), "the initial {initial} is drawn: {texts:?}");
    }
}

#[test]
fn the_character_page_draws_one_32_px_sprite() {
    let site = editing_site();
    let ui = sized_ui(&site);
    ui.navigate(EditorCommand::Party, 0);

    let sizes: Vec<(f32, f32)> = sprites(&ui).iter().map(|element| (element.size().width, element.size().height)).collect();
    // The cards are not instantiated under the page: the one sprite is the
    // page's.
    assert_eq!(sizes, vec![(32.0, 32.0)]);
}
