//! The party overview: one card per character and the gil.
//!
//! This is the one module of the app that names Pixel Remaster fields. The
//! rest of the view model reads metadata and stays blind to the game. A card
//! reads the name, job, level, HP, experience and the eight spell charges of
//! a character (`docs/formats/pixel-remaster.md#characters`) through the
//! constants of `neoffse::pr::ff1::names`, never through a label or a
//! position. The gil comes from the Inventory section.

use neoffse::core::{Capability, Entity, FieldId, FieldView, Game, SaveView, Value};
use neoffse::data::{Language, tables_for};
use neoffse::pr::ff1::names;
use slint::SharedString;

use super::field_row::model_of;
use super::pick::entry_name;
use crate::{CharacterCard, Overview};

/// The party overview of `view`: one card per Party entity in section order,
/// and the gil.
///
/// `present` is false for a game without a card builder (today every game
/// but FF I), and no card is built. A value the view lacks, or holds as
/// another kind, gives an empty string for that entry. `hp_max` and
/// `next_exp` come from the view fields `names::MAX_HP` and
/// `names::NEXT_LEVEL`; `next_exp` is empty at level 99, where no next row
/// exists.
///
/// The result holds names and numbers from the save: never log it. Builds
/// Slint models: UI thread only.
#[must_use]
pub fn overview(view: &SaveView, game: Game, language: Language) -> Overview {
    match game {
        Game::Ff1Pr => ff1_overview(view, game, language),
        _ => Overview::default(),
    }
}

fn ff1_overview(view: &SaveView, game: Game, language: Language) -> Overview {
    let cards: Vec<CharacterCard> = view
        .section(Capability::Party)
        .map(|section| {
            section
                .entities
                .iter()
                .enumerate()
                .map(|(index, entity)| card(entity, index, game, language))
                .collect()
        })
        .unwrap_or_default();
    Overview {
        present: true,
        cards: model_of(cards),
        gil: gil(view).into(),
    }
}

/// The card of the entity at `index` among the Party entities. In FF I every
/// Party entity is a page, so `index` is the page index.
fn card(entity: &Entity, index: usize, game: Game, language: Language) -> CharacterCard {
    let field = |name: &'static str| find(entity, &entity.id.clone().with_name(name));
    let charges: Vec<SharedString> = (1..=names::SPELL_LEVELS)
        .map(|level| number(find(entity, &entity.id.clone().with_name(names::CHARGES).with_id(level))).into())
        .collect();
    CharacterCard {
        page: i32::try_from(index).unwrap_or(i32::MAX),
        name: entity.label.as_str().into(),
        initial: initial(&entity.label).into(),
        job: job(field(names::JOB), game, language).into(),
        level: number(field(names::LEVEL)).into(),
        hp: number(field(names::HP)).into(),
        hp_max: number(field(names::MAX_HP)).into(),
        charges: model_of(charges),
        exp: number(field(names::EXP)).into(),
        next_exp: number(field(names::NEXT_LEVEL)).into(),
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

/// The first letter of `label`, upper case. `""` for an empty label.
fn initial(label: &str) -> String {
    label.chars().next().map(|first| first.to_uppercase().collect()).unwrap_or_default()
}

/// The text of the gil field of the Inventory section, `""` when the section
/// or the field is absent.
fn gil(view: &SaveView) -> String {
    let id = FieldId::new(names::GIL);
    let field = view
        .section(Capability::Inventory)
        .and_then(|section| section.entities.iter().find_map(|entity| find(entity, &id)));
    number(field)
}
