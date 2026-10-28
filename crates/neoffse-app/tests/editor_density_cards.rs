//! The party cards at the dense text size: nothing clips at 800 by 600 or at
//! 1280 by 800, the level row keeps a gap, and a card is as high as its rows.
//! Geometry comes from the accessibility tree of Slint's testing backend; no
//! window shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fmt;
use std::sync::Arc;

use common::{Card, Rect, SLACK, Ui, cards_ui, editing_site, open_editor, resize, visible_cards};
use i_slint_backend_testing::ElementHandle;
use neoffse_app::{ProviderChain, Theme, refresh_editor};
use slint::ComponentHandle;

/// The two window sizes the editor must fit: the 800 x 600 minimum and 1280 x 800.
const SIZES: [(f32, f32); 2] = [(800.0, 600.0), (1280.0, 800.0)];

/// A window size, with the bar idle or with the level-up notice and a pending change.
#[derive(Debug, Clone, Copy)]
struct State {
    width: f32,
    height: f32,
    notice: bool,
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}{}", self.width, self.height, if self.notice { " with the notice" } else { "" })
    }
}

/// Runs `check` on the editor's cards at each size, first with the pending
/// bar idle, then with the level-up notice and one pending change. The bar
/// shows in both. One window serves all four states: the testing backend
/// installs its platform once per thread.
fn in_each_state(mut check: impl FnMut(&Ui, State)) {
    let site = editing_site();
    let ui = open_editor(&site);
    for notice in [false, true] {
        if notice {
            ui.make_notice();
        } else {
            assert!(ElementHandle::find_by_element_type_name(&ui.window, "PendingBar").next().is_some());
            assert_eq!(ui.bridge().get_pending_count(), 0);
        }
        for (width, height) in SIZES {
            resize(&ui, width, height);
            assert!(ui.on_cards());
            check(&ui, State { width, height, notice });
        }
    }
}

fn panel(ui: &Ui) -> Rect {
    Rect::of(
        &ElementHandle::find_by_element_type_name(&ui.window, "PartyCards")
            .next()
            .expect("the party cards panel"),
    )
}

fn whole_cards(ui: &Ui) -> Vec<Card> {
    visible_cards(ui).into_iter().filter(Card::whole).collect()
}

/// The Level value text: the first text on the "Lv" row right of the label.
fn level_value(card: &Card) -> Rect {
    let label = card.text("Lv");
    card.texts
        .iter()
        .map(|(_, rect)| *rect)
        .filter(|rect| (rect.y - label.y).abs() < SLACK && rect.x > label.x + SLACK)
        .min_by(|a, b| a.x.total_cmp(&b.x))
        .expect("a level value")
}

#[test]
fn no_card_clips_at_800_or_1280() {
    in_each_state(|ui, state| {
        let panel = panel(ui);
        let cards = visible_cards(ui);
        assert!(!cards.is_empty(), "{state}: a card shows");
        for card in &cards {
            assert!(
                card.bounds.x >= panel.x - SLACK && card.bounds.right() <= panel.right() + SLACK,
                "{state}, {}: the card lies inside the panel horizontally ({:?} in {panel:?})",
                card.name,
                card.bounds
            );
            for (label, text) in &card.texts {
                assert!(
                    card.bounds.holds(text),
                    "{state}, {}: the text {label:?} lies inside the card ({text:?} in {:?})",
                    card.name,
                    card.bounds
                );
            }
        }
    });
}

#[test]
fn the_level_row_keeps_a_gap() {
    in_each_state(|ui, state| {
        let cards = whole_cards(ui);
        assert!(!cards.is_empty(), "{state}: a whole card shows");
        for card in &cards {
            let gap = card.text("Next level").x - level_value(card).right();
            assert!(gap >= 8.0, "{state}, {}: the level value ends {gap} px before \"Next level\"", card.name);
        }
    });
}

#[test]
fn four_cards_fit_when_the_window_allows() {
    in_each_state(|ui, state| {
        let whole = whole_cards(ui).len();
        if state.width < 1000.0 && state.notice {
            assert!(whole >= 3, "{state} shows {whole} whole cards");
        } else {
            assert_eq!(whole, 4, "{state}");
        }
    });
}

#[test]
fn a_card_is_as_high_as_its_rows() {
    let ui = cards_ui(1280.0, 1200.0);
    let padding = ui.window.global::<Theme>().get_look().padding;
    let cards = visible_cards(&ui);
    assert_eq!(cards.len(), 4);
    for card in &cards {
        let frame = card.frame.expect("a portrait frame").h;
        let line = card.text("HP").h;
        let expected = (frame + 2.0 * padding).max(4.0 * line + 2.0 * padding);
        assert!(
            (card.bounds.h - expected).abs() <= 1.0,
            "{}: {} px high, its rows ask {expected} px",
            card.name,
            card.bounds.h
        );
    }
}

#[test]
fn a_card_without_a_portrait_keeps_its_height() {
    let ui = cards_ui(1280.0, 1200.0);
    let before = visible_cards(&ui);
    ui.app.editor.set_assets(Arc::new(ProviderChain::new(Vec::new())));
    refresh_editor(&ui.window, &ui.app);

    let after = visible_cards(&ui);
    assert_eq!(after.len(), before.len());
    for (with, without) in before.iter().zip(&after) {
        assert!(
            (with.bounds.h - without.bounds.h).abs() < SLACK,
            "{} keeps its height without a sprite",
            with.name
        );
        assert!(without.sprite.is_none(), "{} draws no sprite", without.name);
        for frame in [with.frame, without.frame] {
            let frame = frame.expect("a portrait frame");
            assert_eq!((frame.w, frame.h), (68.0, 68.0), "{} keeps the 68 px frame", with.name);
        }
    }
}

#[test]
fn the_portrait_frame_is_a_whole_multiple_of_the_sprite() {
    let ui = cards_ui(1280.0, 800.0);
    let cards = visible_cards(&ui);
    assert!(!cards.is_empty());
    for card in &cards {
        let side = card.sprite.expect("a sprite");
        assert!(
            side.w >= 32.0 && (side.w / 32.0).fract() == 0.0,
            "{}: the sprite is {} px wide",
            card.name,
            side.w
        );
        assert!((side.w - side.h).abs() < SLACK, "{}: the sprite is square", card.name);
    }
}
