//! The Party panel fits its cards like the game's menu window: the margin
//! under the last card equals the margin above the first, the panel stops
//! there, the status panel ends level with it, and a window too small for
//! every card scrolls them. Every other page keeps the full-height row. Geometry comes from the accessibility
//! tree of Slint's testing backend; no window shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Rect, SLACK, Ui, editing_site, open_editor, resize, visible_cards};
use i_slint_backend_testing::ElementHandle;
use neoffse_app::EditorCommand;

/// The right-hand panel: the `Frame` farthest right (the command list and
/// the status panel are frames too).
fn page_panel(ui: &Ui) -> Rect {
    ElementHandle::find_by_element_type_name(&ui.window, "Frame")
        .map(|panel| Rect::of(&panel))
        .max_by(|a, b| a.x.total_cmp(&b.x))
        .expect("the page panel")
}

fn status_panel(ui: &Ui) -> Rect {
    Rect::of(
        &ElementHandle::find_by_element_type_name(&ui.window, "StatusPanel")
            .next()
            .expect("the status panel"),
    )
}

/// The editor on the cards at `width` by `height`.
fn cards_ui(width: f32, height: f32) -> Ui {
    let site = editing_site();
    let ui = open_editor(&site);
    resize(&ui, width, height);
    assert!(ui.on_cards());
    ui
}

#[test]
fn the_margin_under_the_last_card_equals_the_margin_above_the_first() {
    let ui = cards_ui(1280.0, 800.0);
    let panel = page_panel(&ui);
    let cards = visible_cards(&ui);
    assert_eq!(cards.len(), 4, "every card shows");
    let top = cards[0].bounds.y - panel.y;
    let bottom = panel.bottom() - cards[3].bounds.bottom();
    assert!(top > 0.0, "the first card lies inside the panel ({top} px)");
    assert!((top - bottom).abs() <= SLACK, "{top} px above the first card, {bottom} px under the last");
}

/// The cards panel, the status panel and the editor window's content height.
fn bottoms(ui: &Ui) -> (f32, f32) {
    (page_panel(ui).bottom(), status_panel(ui).bottom())
}

#[test]
fn the_panel_is_shorter_than_the_window_and_the_status_panel_ends_with_it() {
    let ui = cards_ui(1280.0, 800.0);
    let (panel, status) = bottoms(&ui);
    assert!((panel - status).abs() <= SLACK, "the panel ends at {panel}, the status panel at {status}");
    assert!(panel < 800.0 - 100.0, "the row stops under the cards ({panel} of 800 px)");
}

#[test]
fn the_status_panel_ends_with_the_cards_at_800_by_600() {
    let ui = cards_ui(800.0, 600.0);
    let (panel, status) = bottoms(&ui);
    assert!((panel - status).abs() <= SLACK, "the panel ends at {panel}, the status panel at {status}");
}

#[test]
fn a_small_window_scrolls_the_cards_and_both_bottoms_line_up() {
    let site = editing_site();
    let ui = open_editor(&site);
    ui.commit(ui.level().key, "34");
    assert!(ui.bridge().get_has_notice());
    assert_eq!(ui.bridge().get_pending_count(), 1);
    resize(&ui, 800.0, 600.0);
    assert!(ui.on_cards());
    let (panel, status) = bottoms(&ui);
    assert!((panel - status).abs() <= SLACK, "the panel ends at {panel}, the status panel at {status}");
    let cards = visible_cards(&ui);
    let whole = cards.iter().filter(|card| card.whole()).count();
    assert!(whole >= 3, "{whole} whole cards show");
    let last = cards.last().expect("a card");
    assert!(!last.whole() || cards.len() < 4, "the fourth card is cut or hidden: the list scrolls");
}

#[test]
fn the_other_pages_keep_the_full_height_row() {
    let ui = cards_ui(1280.0, 800.0);
    let on_cards = status_panel(&ui).bottom();
    for (command, character) in [(EditorCommand::Party, 0), (EditorCommand::Items, -1), (EditorCommand::Magic, -1)] {
        ui.navigate(command, character);
        let (panel, status) = (page_panel(&ui), status_panel(&ui));
        assert!(
            status.bottom() > on_cards + 10.0,
            "{command:?} {character}: the row stretches ({} against {on_cards})",
            status.bottom()
        );
        assert!(
            (panel.bottom() - status.bottom()).abs() <= SLACK,
            "{command:?} {character}: the panel ends at {}, the status panel at {}",
            panel.bottom(),
            status.bottom()
        );
    }
}

#[test]
fn the_panel_fits_again_after_a_visit_elsewhere() {
    let ui = cards_ui(1280.0, 800.0);
    let before = page_panel(&ui);
    ui.navigate(EditorCommand::Party, 0);
    ui.navigate(EditorCommand::Party, -1);
    let after = page_panel(&ui);
    assert!((before.h - after.h).abs() <= SLACK, "{} px high before, {} after", before.h, after.h);
}
