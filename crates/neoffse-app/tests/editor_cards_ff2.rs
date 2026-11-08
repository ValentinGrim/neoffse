//! The FF II card on the testing backend: no Lv, Exp or Next level text, HP and
//! MP shown, a card as high as an FF I card and the same pitch between
//! portraits in both games. The Magic page lists the Level
//! fields by spell name and a Level commit is one undoable edit. No window
//! shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::sync::Arc;

use common::{Part, SLACK, Ui, cards_ui, ff2_editing, ff2_site, ff2_ui, field, fields, page, resize, visible_cards};
use neoffse::core::{FieldId, Value};
use neoffse::pr::ff2::names;
use neoffse_app::editor::{Input, value_for};
use neoffse_app::{EditProblem, ProviderChain, Theme, refresh_editor};
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

/// The window sizes the card geometry holds at: the 800 by 600 minimum and a
/// tall window. One window per game is resized through them: the testing
/// backend installs its platform once per thread.
const SIZES: [(f32, f32); 2] = [(800.0, 600.0), (1280.0, 1200.0)];

fn padding(ui: &Ui) -> f32 {
    ui.window.global::<Theme>().get_look().padding
}

/// Runs `check` on the cards in view at each size, with the card padding. At
/// least two cards must show.
fn at_each_size(ui: &Ui, mut check: impl FnMut(&[common::Card], f32)) {
    let padding = padding(ui);
    for (width, height) in SIZES {
        resize(ui, width, height);
        let cards = visible_cards(ui);
        assert!(cards.len() >= 2, "{width}x{height}: {} cards in view", cards.len());
        check(&cards, padding);
    }
}

#[test]
fn the_ff1_card_height_is_unchanged() {
    let ui = cards_ui(1280.0, 1200.0);
    at_each_size(&ui, |cards, padding| {
        for card in cards {
            let expected = ff1_card_height(card, padding);
            assert!((card.bounds.h - expected).abs() <= 1.0, "{}: {} against {expected}", card.name, card.bounds.h);
        }
    });
}

#[test]
fn the_ff1_card_is_the_four_rows_or_the_frame() {
    let ui = cards_ui(1280.0, 1200.0);
    at_each_size(&ui, |cards, padding| {
        for card in cards {
            let rows = 4.0 * card.text("HP").h + 2.0 * padding;
            let frame = card.frame.expect("a portrait frame").h + 2.0 * padding;
            assert!(
                (card.bounds.h - rows.max(frame)).abs() <= SLACK,
                "{}: {} against max({rows}, {frame})",
                card.name,
                card.bounds.h
            );
        }
    });
}

#[test]
fn an_ff2_card_is_as_high_as_an_ff1_card() {
    let ui = ff2_cards_ui(1280.0, 1200.0);
    at_each_size(&ui, |cards, padding| {
        for card in cards {
            // The old layout passes this test unless the four rows are the
            // taller term.
            let rows = 4.0 * card.text("HP").h + 2.0 * padding;
            assert!(
                rows > card.frame.expect("a portrait frame").h + 2.0 * padding,
                "the four rows decide the height"
            );
            let expected = ff1_card_height(card, padding);
            assert!((card.bounds.h - expected).abs() <= SLACK, "{}: {} against {expected}", card.name, card.bounds.h);
            assert!(
                card.bounds.h >= card.frame.expect("a portrait frame").h + 2.0 * padding - SLACK,
                "{} holds its frame",
                card.name
            );
        }
    });
}

/// The distances from each card's top to the next card's top, the last card
/// having none.
fn pitches(cards: &[common::Card]) -> Vec<f32> {
    cards.windows(2).map(|pair| pair[1].bounds.y - pair[0].bounds.y).collect()
}

/// At each size the pitch between successive cards is each card's height (the
/// column has no spacing) and equals the FF I card height. Two windows cannot
/// share a thread, so each game runs its own test and the shared formula ties
/// them together.
fn assert_pitch_is_the_ff1_height(ui: &Ui) {
    at_each_size(ui, |cards, padding| {
        for (card, gap) in cards.iter().zip(pitches(cards)) {
            assert!(
                (gap - card.bounds.h).abs() <= SLACK,
                "{}: pitch {gap} against its height {}",
                card.name,
                card.bounds.h
            );
            let expected = ff1_card_height(card, padding);
            assert!((gap - expected).abs() <= SLACK, "{}: pitch {gap} against {expected}", card.name);
        }
    });
}

#[test]
fn the_pitch_between_ff2_cards_is_an_ff1_card_height() {
    assert_pitch_is_the_ff1_height(&ff2_cards_ui(1280.0, 1200.0));
}

#[test]
fn the_pitch_between_ff1_cards_is_an_ff1_card_height() {
    assert_pitch_is_the_ff1_height(&cards_ui(1280.0, 1200.0));
}

#[test]
fn the_ff2_portrait_and_rows_stay_centred_in_their_card() {
    let ui = ff2_cards_ui(1280.0, 1200.0);
    at_each_size(&ui, |cards, padding| {
        for card in cards {
            let frame = card.frame.expect("a portrait frame");
            assert_eq!(frame.h, 68.0, "{}", card.name);
            let (top, bottom) = (frame.y - card.bounds.y, card.bounds.bottom() - frame.bottom());
            assert!((top - bottom).abs() <= SLACK, "{}: gaps {top} and {bottom} around the frame", card.name);
            assert!(top >= padding - SLACK, "{}: gap {top} under the padding {padding}", card.name);
            let (hp_top, mp_bottom) = (card.text("HP").y - card.bounds.y, card.bounds.bottom() - card.text("MP").bottom());
            assert!((hp_top - mp_bottom).abs() <= SLACK, "{}: HP gap {hp_top} against MP gap {mp_bottom}", card.name);
        }
    });
}

#[test]
fn an_ff2_card_without_a_portrait_keeps_its_height() {
    let ui = ff2_cards_ui(1280.0, 1200.0);
    ui.app.editor.set_assets(Arc::new(ProviderChain::new(Vec::new())));
    refresh_editor(&ui.window, &ui.app);
    at_each_size(&ui, |cards, padding| {
        for card in cards {
            assert!(card.sprite.is_none(), "{} draws no sprite", card.name);
            let expected = ff1_card_height(card, padding);
            assert!((card.bounds.h - expected).abs() <= SLACK, "{}: {} against {expected}", card.name, card.bounds.h);
        }
    });
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
