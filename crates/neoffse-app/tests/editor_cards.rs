//! The party cards: each keeps the height of its rows, the panel scrolls when
//! it is short, and a focused card scrolls into view. Geometry comes from the
//! accessibility tree of Slint's testing backend; no window shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Ui, editing_site, open_editor};
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use slint::platform::Key;
use slint::{ComponentHandle, LogicalSize};

const NAMES: [&str; 4] = ["Aria", "Bram", "Cleo", "Zoé"];
const SLACK: f32 = 0.5;
#[derive(Debug, Clone, Copy, PartialEq)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

impl Rect {
    fn of(element: &ElementHandle) -> Self {
        let (position, size) = (element.absolute_position(), element.size());
        Self {
            x: position.x,
            y: position.y,
            w: size.width,
            h: size.height,
        }
    }

    fn bottom(&self) -> f32 {
        self.y + self.h
    }

    fn holds(&self, inner: &Self) -> bool {
        inner.x >= self.x - SLACK && inner.y >= self.y - SLACK && inner.x + inner.w <= self.x + self.w + SLACK && inner.bottom() <= self.bottom() + SLACK
    }
}

/// A card the testing backend can see: its bounds and every text inside it.
/// A card clipped away by the scroll panel is absent.
struct Card {
    name: String,
    bounds: Rect,
    texts: Vec<Rect>,
}

fn resize(ui: &Ui, width: f32, height: f32) {
    ui.window.window().set_size(LogicalSize::new(width, height));
}

/// The cards in view, top to bottom.
fn visible_cards(ui: &Ui) -> Vec<Card> {
    ElementHandle::find_by_element_type_name(&ui.window, "CharacterRow")
        .map(|row| {
            let button = row
                .query_descendants()
                .match_accessible_role(AccessibleRole::Button)
                .find_first()
                .expect("a focus scope");
            let texts = row
                .query_descendants()
                .match_accessible_role(AccessibleRole::Text)
                .find_all()
                .iter()
                .map(Rect::of)
                .collect();
            Card {
                name: button.accessible_label().expect("a name").to_string(),
                bounds: Rect::of(&row),
                texts,
            }
        })
        .collect()
}

/// The editor on the cards, with the level-up notice and the bar shown.
fn cards_with_notice_and_bar(height: f32) -> Ui {
    let site = editing_site();
    let ui = open_editor(&site);
    resize(&ui, 1280.0, height);
    ui.commit(ui.level().key, "34");
    assert!(ui.bridge().get_has_notice());
    assert_eq!(ui.bridge().get_pending_count(), 1);
    assert!(ui.on_cards());
    ui
}

/// Checks the geometry of what is in view and records the checked cards and
/// adjacent pairs.
fn check(cards: &[Card], seen: &mut Vec<String>, pairs: &mut Vec<(String, String)>) {
    for card in cards {
        assert!(card.bounds.h > 0.0, "{} has a height", card.name);
        for (index, text) in card.texts.iter().enumerate() {
            assert!(card.bounds.holds(text), "text {index} of {} lies inside its card", card.name);
        }
        // A card cut by the panel edge lists fewer texts: only whole cards count.
        if card.texts.len() >= 10 {
            seen.push(card.name.clone());
        }
    }
    for pair in cards.windows(2) {
        assert!(
            pair[1].bounds.y >= pair[0].bounds.bottom() - SLACK,
            "{} starts at or below the end of {}",
            pair[1].name,
            pair[0].name
        );
        pairs.push((pair[0].name.clone(), pair[1].name.clone()));
    }
}

#[test]
fn no_two_cards_overlap_and_each_holds_its_rows() {
    let ui = cards_with_notice_and_bar(800.0);
    let (mut seen, mut pairs) = (Vec::new(), Vec::new());
    check(&visible_cards(&ui), &mut seen, &mut pairs);
    // Tab walks the focus down the cards and scrolls the panel with it.
    for _ in 0..40 {
        ui.key(Key::Tab);
        check(&visible_cards(&ui), &mut seen, &mut pairs);
    }

    for name in NAMES {
        assert!(seen.iter().any(|seen| seen == name), "{name} was checked whole");
    }
    for pair in NAMES.windows(2) {
        assert!(
            pairs.iter().any(|(a, b)| a == pair[0] && b == pair[1]),
            "{} and {} were checked together",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn a_card_keeps_its_height_when_the_panel_is_short() {
    let ui = cards_with_notice_and_bar(1200.0);
    let tall = visible_cards(&ui);
    assert_eq!(tall.len(), 4, "a tall window shows the four cards");
    let pitch = tall[1].bounds.y - tall[0].bounds.y;
    assert!((pitch - tall[0].bounds.h).abs() < SLACK, "the cards touch, nothing is squeezed");

    resize(&ui, 1280.0, 560.0);
    let short = visible_cards(&ui);
    assert!(short.len() < 4, "a short panel clips the last cards");
    for card in &short {
        assert!((card.bounds.h - tall[0].bounds.h).abs() < SLACK, "{} keeps its height", card.name);
    }
}

#[test]
fn tabbing_to_the_last_card_scrolls_it_into_view() {
    let ui = cards_with_notice_and_bar(800.0);
    assert!(!visible_cards(&ui).iter().any(|card| card.name == "Zoé"), "the last card starts out of view");

    let mut reached = None;
    for _ in 0..40 {
        ui.key(Key::Tab);
        if let Some(card) = visible_cards(&ui).into_iter().find(|card| card.name == "Zoé") {
            reached = Some(card.bounds);
            break;
        }
    }

    assert!(reached.is_some(), "the last card scrolled into view");
}

#[test]
fn the_notice_and_the_bar_do_not_change_a_card() {
    let site = editing_site();
    let ui = open_editor(&site);
    resize(&ui, 1280.0, 1200.0);
    let clean = visible_cards(&ui);
    ui.commit(ui.level().key, "34");
    let both = visible_cards(&ui);

    assert_eq!((clean.len(), both.len()), (4, 4));
    for (clean, both) in clean.iter().zip(&both) {
        assert!((clean.bounds.h - both.bounds.h).abs() < SLACK, "the height of {} does not change", clean.name);
    }
}
