//! The FF II card on the testing backend: no Lv, Exp or Next level text, HP and
//! MP shown, a card no taller than an FF I card. The Magic page lists the Level
//! fields by spell name and a Level commit is one undoable edit. No window
//! shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Part, Ui, cards_ui, ff2_editing, ff2_site, ff2_ui, field, fields, page, resize, visible_cards};
use neoffse::core::{FieldId, Value};
use neoffse::pr::ff2::names;
use neoffse_app::editor::{Input, value_for};
use neoffse_app::{EditProblem, Theme};
use slint::ComponentHandle;

fn ff2_cards_ui(width: f32, height: f32) -> Ui {
    let site = ff2_site();
    let ui = ff2_ui(&site);
    resize(&ui, width, height);
    assert!(ui.on_cards());
    ui
}

fn texts(card: &common::Card) -> Vec<String> {
    card.texts.iter().map(|(text, _)| text.clone()).collect()
}

#[test]
fn an_ff2_card_has_no_level_row() {
    let ui = ff2_cards_ui(1280.0, 800.0);
    let cards = visible_cards(&ui);
    assert_eq!(cards.len(), 4);
    for card in &cards {
        let shown = texts(card);
        for label in ["Lv", "Exp", "Next level"] {
            assert!(!shown.iter().any(|text| text == label), "{} shows {label}", card.name);
        }
    }
}

#[test]
fn an_ff2_card_shows_hp_and_mp_with_their_maximum() {
    let ui = ff2_cards_ui(1280.0, 800.0);
    let cards = visible_cards(&ui);
    for card in &cards {
        let shown = texts(card);
        assert!(shown.iter().any(|text| text == "HP"), "{} has no HP label", card.name);
        assert!(shown.iter().any(|text| text == "MP"), "{} has no MP label", card.name);
    }
    let first = texts(&cards[0]);
    assert!(first.iter().any(|text| text == "140/ 150"), "{first:?}");
    assert!(first.iter().any(|text| text == "20/ 24"), "{first:?}");
}

#[test]
fn an_ff2_card_has_one_mp_row() {
    let ui = ff2_cards_ui(1280.0, 800.0);
    for card in visible_cards(&ui) {
        let count = texts(&card).iter().filter(|text| *text == "MP").count();
        assert_eq!(count, 1, "{}", card.name);
    }
}

#[test]
fn an_ff1_card_still_shows_lv() {
    let ui = cards_ui(1280.0, 800.0);
    for card in visible_cards(&ui) {
        let shown = texts(&card);
        assert!(shown.iter().any(|text| text == "Lv"), "{}", card.name);
        assert_eq!(shown.iter().filter(|text| *text == "MP").count(), 1, "{}", card.name);
    }
}

/// The height an FF I card takes: its portrait frame or its four text rows,
/// whichever is higher, plus the padding. The density tests pin it.
fn ff1_card_height(card: &common::Card, padding: f32) -> f32 {
    let frame = card.frame.expect("a portrait frame").h;
    (frame + 2.0 * padding).max(4.0 * card.text("HP").h + 2.0 * padding)
}

#[test]
fn the_ff1_card_height_is_unchanged() {
    let ui = cards_ui(1280.0, 1200.0);
    let padding = ui.window.global::<Theme>().get_look().padding;
    let cards = visible_cards(&ui);
    assert_eq!(cards.len(), 4);
    for card in &cards {
        let expected = ff1_card_height(card, padding);
        assert!((card.bounds.h - expected).abs() <= 1.0, "{}: {} against {expected}", card.name, card.bounds.h);
    }
}

#[test]
fn an_ff2_card_is_no_taller_than_an_ff1_card() {
    let ui = ff2_cards_ui(1280.0, 1200.0);
    let padding = ui.window.global::<Theme>().get_look().padding;
    let cards = visible_cards(&ui);
    assert_eq!(cards.len(), 4);
    for card in &cards {
        // An FF I card of this look and font is at least this high.
        let ceiling = ff1_card_height(card, padding);
        assert!(card.bounds.h <= ceiling + 1.0, "{}: {} against {ceiling}", card.name, card.bounds.h);
    }
}

#[test]
fn the_magic_page_lists_a_level_field_per_spell_name() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let data = ed.data();
    let labels: Vec<String> = fields(&page(&data, Part::Magic, "Ivo")).iter().map(|row| row.label.to_string()).collect();
    assert_eq!(labels, ["Fire", "Thunder", "Blizzard"]);
    assert_eq!(field(&data, Part::Magic, "Ivo", "Fire").text, "5");
    assert!(field(&data, Part::Magic, "Ivo", "Fire").editable);
}

#[test]
fn a_typed_99_clamps_to_16() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let key = field(&ed.data(), Part::Magic, "Ivo", "Fire").key;
    let view = ed.with_session(|session| session.view().unwrap());
    let level = FieldId::new(names::CHARACTER).with_id(1).with_name(names::SPELL).with_id(0);
    let (_, level) = view.find(&level).expect("the first chain is Ivo's Fire");
    let meta = level.meta.clone();
    assert_eq!(value_for(&meta, &Input::Text("99".to_owned())), Ok(Value::Int(16)));

    assert!(ed.editor().commit(key, Input::Text("99".to_owned())).is_none());
    assert_eq!(field(&ed.data(), Part::Magic, "Ivo", "Fire").text, "16");
}

#[test]
fn a_level_commit_is_one_edit_and_undo_restores_every_copy() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let before = ed.with_session(|session| session.view().unwrap());
    let key = field(&ed.data(), Part::Magic, "Ivo", "Fire").key;

    assert!(ed.editor().commit(key, Input::Text("9".to_owned())).is_none());
    assert_eq!(ed.pending(), 1);
    assert_eq!(field(&ed.data(), Part::Magic, "Ivo", "Fire").text, "9");

    assert!(ed.editor().undo().is_none());
    assert_eq!(ed.pending(), 0);
    let after = ed.with_session(|session| session.view().unwrap());
    assert!(after == before, "the view differs after the undo");
}

#[test]
fn an_over_cap_power_gain_is_refused_and_offers_expert_mode() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let key = field(&ed.data(), Part::Party, "Ivo", "Power gain").key;
    let notice = ed.editor().commit(key, Input::Text("99999".to_owned())).expect("a refusal");
    assert_eq!(notice.problem, EditProblem::AboveCap);
    assert!(ed.editor().offers_expert(&notice));
    assert_eq!(ed.pending(), 0);
}
