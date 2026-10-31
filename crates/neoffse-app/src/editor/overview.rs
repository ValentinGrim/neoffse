//! The party overview: one card per character and the gil.
//!
//! This is the one module of the app that names Pixel Remaster fields. The
//! rest of the view model reads metadata and stays blind to the game. The job
//! id to image map lives in [`crate::assets`]: this module reaches it through
//! the [`AssetProvider`] trait only. An FF I card reads the name, job, level,
//! HP, experience and the eight spell charges of a character
//! (`docs/formats/pixel-remaster.md#characters`) through the constants of
//! `neoffse::pr::ff1::names`; an FF II card reads the name, HP and MP through
//! those of `neoffse::pr::ff2::names`
//! (`docs/formats/pixel-remaster.md#character-growth-in-ff-ii`). No card
//! reads a label or a position. The gil comes from the Inventory section.

use neoffse::core::{Capability, Entity, FieldId, FieldView, Game, SaveView, Value};
use neoffse::data::{Language, tables_for};
use neoffse::pr::{ff1, ff2};
use slint::{Model, SharedString};

use super::field_row::model_of;
use super::pick::entry_name;
use crate::assets::AssetProvider;
use crate::{CharacterCard, Overview};

/// The party overview of `view`: one card per Party entity in section order,
/// the table that names the card of each party page ([`card_of_page`]), and
/// the gil.
///
/// `present` is false for a game without a card builder (today every game
/// but FF I and FF II), and no card is built and `assets` is never asked. A card holds
/// the portrait `assets` gives for the job id of its character, and the empty
/// image when the job is missing, not a number, outside `u32` or unknown to
/// the provider. A value the view lacks, or holds as another kind, gives an
/// empty string for that entry. `hp_max` and `next_exp` come from the view fields `names::MAX_HP` and
/// `names::NEXT_LEVEL`; `next_exp` is empty at level 99, where no next row
/// exists.
///
/// An FF I card has a level (`has_level`), charges and no `mp`. An FF II card
/// has `mp` and `mp_max` in place of the charges and no level: `has_level` is
/// false, and `level`, `exp`, `next_exp`, `job` and `charges` stay empty. It
/// carries no portrait yet, so the frame draws the initial.
///
/// The result holds names and numbers from the save: never log it. Builds
/// Slint models and images: UI thread only.
#[must_use]
pub fn overview(view: &SaveView, game: Game, language: Language, assets: &dyn AssetProvider) -> Overview {
    match game {
        Game::Ff1Pr => overview_of(view, ff1::names::GIL, |entity, index| ff1_card(entity, index, game, language, assets)),
        Game::Ff2Pr => overview_of(view, ff2::names::GIL, ff2_card),
        _ => Overview::default(),
    }
}

/// The levels the cards gained between two overviews: for each card present
/// in both (matched by `page`), the rise of the level when both levels are
/// numbers and the second is higher. Zero for an overview with `present`
/// false, a missing card, a text that is no number, or a fall.
///
/// It reads `CharacterCard::page` and `CharacterCard::level` only, so it
/// names no game field. The sum saturates at `u32::MAX`.
#[must_use]
pub fn levels_gained(before: &Overview, after: &Overview) -> u32 {
    if !before.present || !after.present {
        return 0;
    }
    let old: Vec<CharacterCard> = before.cards.iter().collect();
    after
        .cards
        .iter()
        .filter_map(|new| {
            let old = old.iter().find(|card| card.page == new.page)?;
            let rise = new.level.parse::<i64>().ok()?.saturating_sub(old.level.parse::<i64>().ok()?);
            (rise > 0).then(|| u32::try_from(rise).unwrap_or(u32::MAX))
        })
        .fold(0, u32::saturating_add)
}

/// For each page `0..pages`, the position in `cards` of the first card whose
/// `page` equals it, `-1` when none does. A card whose `page` is negative or
/// at least `pages` maps to no entry.
#[must_use]
pub fn card_of_page(cards: &[CharacterCard], pages: usize) -> Vec<i32> {
    let mut table = vec![-1; pages];
    for (position, card) in cards.iter().enumerate() {
        let Some(entry) = usize::try_from(card.page).ok().and_then(|page| table.get_mut(page)) else {
            continue;
        };
        if *entry < 0 {
            // No save holds 2^31 cards: the saturation never meets a real position.
            *entry = i32::try_from(position).unwrap_or(i32::MAX);
        }
    }
    table
}

/// The overview of `view`: `make` builds the card of the Party entity at a
/// position, `gil` names the gil field of the Inventory section.
fn overview_of(view: &SaveView, gil_name: &'static str, make: impl Fn(&Entity, usize) -> CharacterCard) -> Overview {
    let section = view.section(Capability::Party);
    let cards: Vec<CharacterCard> = section
        .map(|section| section.entities.iter().enumerate().map(|(index, entity)| make(entity, index)).collect())
        .unwrap_or_default();
    let pages = section.map_or(0, |section| section.entities.len());
    Overview {
        present: true,
        card_of_page: model_of(card_of_page(&cards, pages)),
        cards: model_of(cards),
        gil: gil(view, gil_name).into(),
    }
}

/// The page index of a Party entity as a card's `page`. In both games every
/// Party entity is a page, so the index is the page index.
fn page_of(index: usize) -> i32 {
    // No save holds 2^31 rows: the saturation never meets a real index.
    i32::try_from(index).unwrap_or(i32::MAX)
}

/// The FF I card of the entity at `index` among the Party entities. The
/// portrait follows the job id of the entity, never `index` or the name.
fn ff1_card(entity: &Entity, index: usize, game: Game, language: Language, assets: &dyn AssetProvider) -> CharacterCard {
    let field = |name: &'static str| find(entity, &entity.id.clone().with_name(name));
    let charges: Vec<SharedString> = (1..=ff1::names::SPELL_LEVELS)
        .map(|level| number(find(entity, &entity.id.clone().with_name(ff1::names::CHARGES).with_id(level))).into())
        .collect();
    let portrait = job_id(field(ff1::names::JOB)).and_then(|id| assets.portrait(game, id)).unwrap_or_default();
    CharacterCard {
        page: page_of(index),
        name: entity.label.as_str().into(),
        initial: initial(&entity.label).into(),
        portrait,
        job: job(field(ff1::names::JOB), game, language).into(),
        level: number(field(ff1::names::LEVEL)).into(),
        hp: number(field(ff1::names::HP)).into(),
        hp_max: number(field(ff1::names::MAX_HP)).into(),
        mp: SharedString::new(),
        mp_max: SharedString::new(),
        has_level: true,
        charges: model_of(charges),
        exp: number(field(ff1::names::EXP)).into(),
        next_exp: number(field(ff1::names::NEXT_LEVEL)).into(),
    }
}

/// The FF II card of the entity at `index`: name, HP and MP. FF II stores no
/// level, experience or charges the editor shows
/// (`docs/formats/pixel-remaster.md#character-growth-in-ff-ii`), so those
/// entries stay empty and `has_level` is false.
fn ff2_card(entity: &Entity, index: usize) -> CharacterCard {
    let field = |name: &'static str| find(entity, &entity.id.clone().with_name(name));
    CharacterCard {
        page: page_of(index),
        name: entity.label.as_str().into(),
        initial: initial(&entity.label).into(),
        hp: number(field(ff2::names::HP)).into(),
        hp_max: number(field(ff2::names::MAX_HP)).into(),
        mp: number(field(ff2::names::MP)).into(),
        mp_max: number(field(ff2::names::MAX_MP)).into(),
        has_level: false,
        ..CharacterCard::default()
    }
}

fn find<'a>(entity: &'a Entity, id: &FieldId) -> Option<&'a FieldView> {
    entity.fields.iter().find(|field| field.id == *id)
}

/// The decimal text of an integer field, `""` for a missing field or a value
/// of another kind.
fn number(field: Option<&FieldView>) -> String {
    match field.map(|field| &field.value) {
        Some(Value::Int(value)) => value.to_string(),
        _ => String::new(),
    }
}

/// The job name through the field's own tables, `""` when the id names no
/// entry.
fn job(field: Option<&FieldView>, game: Game, language: Language) -> String {
    let Some(field) = field else { return String::new() };
    let Value::Int(id) = field.value else { return String::new() };
    tables_for(game)
        .and_then(|tables| entry_name(tables, &field.meta.tables, language, id))
        .unwrap_or_default()
}

/// The job id of the job field, `None` for a missing field, a value of
/// another kind or an id outside `u32`.
fn job_id(field: Option<&FieldView>) -> Option<u32> {
    match field?.value {
        Value::Int(id) => u32::try_from(id).ok(),
        _ => None,
    }
}

/// The first letter of `label`, upper case. `""` for an empty label.
fn initial(label: &str) -> String {
    label.chars().next().map(|first| first.to_uppercase().collect()).unwrap_or_default()
}

/// The text of the field `name` of the Inventory section, `""` when the
/// section or the field is absent.
fn gil(view: &SaveView, name: &'static str) -> String {
    let id = FieldId::new(name);
    let field = view
        .section(Capability::Inventory)
        .and_then(|section| section.entities.iter().find_map(|entity| find(entity, &id)));
    number(field)
}
