//! The character page header: a 68 px portrait frame around the 64 px sprite, like the cards', the
//! name over the job beside it, the fields below, and a list that still
//! shows its rows at 800 by 600. Geometry comes from the accessibility tree
//! of Slint's testing backend; no window shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::sync::Arc;

use common::{Rect, SLACK, Ui, editing_site, open_editor, resize};
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use neoffse_app::{EditorCommand, ProviderChain, refresh_editor};
use slint::Model;

const SPRITE: &str = "PortraitFrame::sprite";

/// The editor on the cards at `width` by `height`.
fn cards_ui(width: f32, height: f32) -> Ui {
    let site = editing_site();
    let ui = open_editor(&site);
    resize(&ui, width, height);
    ui
}

fn rects(ui: &Ui, type_name: &str) -> Vec<Rect> {
    ElementHandle::find_by_element_type_name(&ui.window, type_name)
        .map(|element| Rect::of(&element))
        .collect()
}

fn the_rect(ui: &Ui, type_name: &str) -> Rect {
    *rects(ui, type_name).first().unwrap_or_else(|| panic!("no element of type {type_name}"))
}

fn the_element(ui: &Ui, type_name: &str) -> ElementHandle {
    ElementHandle::find_by_element_type_name(&ui.window, type_name)
        .next()
        .unwrap_or_else(|| panic!("no element of type {type_name}"))
}

fn sprite_sizes(ui: &Ui) -> Vec<Rect> {
    ElementHandle::find_by_element_id(&ui.window, SPRITE)
        .map(|element| Rect::of(&element))
        .collect()
}

/// The texts of `element` with their labels, top to bottom.
fn texts_of(element: &ElementHandle) -> Vec<(String, Rect)> {
    let mut found: Vec<(String, Rect)> = element
        .query_descendants()
        .match_accessible_role(AccessibleRole::Text)
        .find_all()
        .iter()
        .map(|text| (text.accessible_label().map(|label| label.to_string()).unwrap_or_default(), Rect::of(text)))
        .collect();
    found.sort_by(|a, b| a.1.y.total_cmp(&b.1.y));
    found
}

#[test]
fn the_header_frame_is_as_large_as_a_card_frame() {
    let ui = cards_ui(1280.0, 800.0);
    let on_cards = sprite_sizes(&ui);
    assert!(!on_cards.is_empty(), "the cards draw sprites");
    ui.navigate(EditorCommand::Party, 0);
    let on_page = sprite_sizes(&ui);
    assert_eq!(on_page.len(), 1, "the page draws one sprite");
    let (card, page) = (on_cards[0], on_page[0]);
    assert!(
        (card.w - page.w).abs() <= SLACK && (card.h - page.h).abs() <= SLACK,
        "card {card:?}, page {page:?}"
    );
    assert!((page.w - page.h).abs() <= SLACK, "the sprite is square: {page:?}");
    assert!((page.w / 32.0 - (page.w / 32.0).round()).abs() <= 0.01, "{} px is not a multiple of 32", page.w);
}

#[test]
fn the_header_text_sits_beside_the_frame() {
    let ui = cards_ui(1280.0, 800.0);
    ui.navigate(EditorCommand::Party, 0);
    let frame = the_rect(&ui, "PortraitFrame");
    let job = ui.app.editor.overview().cards.row_data(0).expect("a card").job.to_string();
    assert!(!job.is_empty(), "the first character has a job");
    let page = the_element(&ui, "CharacterEditPage");
    let texts = texts_of(&page);
    for label in ["Aria", job.as_str()] {
        let (_, text) = texts
            .iter()
            .find(|(text, _)| text == label)
            .unwrap_or_else(|| panic!("no text {label}: {texts:?}"));
        assert!(
            text.x >= frame.right() - SLACK,
            "{label} starts at {} inside the frame ending at {}",
            text.x,
            frame.right()
        );
        assert!(
            text.y >= frame.y - SLACK && text.bottom() <= frame.bottom() + SLACK,
            "{label} ({text:?}) leaves the frame's extent ({frame:?})"
        );
    }
}

#[test]
fn the_first_field_starts_below_the_header() {
    let ui = cards_ui(1280.0, 800.0);
    ui.navigate(EditorCommand::Party, 0);
    let frame = the_rect(&ui, "PortraitFrame");
    let page = the_rect(&ui, "CharacterEditPage");
    let mut rules: Vec<Rect> = rects(&ui, "RuleLine").into_iter().filter(|rule| page.holds(rule)).collect();
    rules.sort_by(|a, b| a.y.total_cmp(&b.y));
    let rule = *rules.first().expect("a rule on the page");
    assert!(
        rule.y >= frame.bottom() - SLACK,
        "the rule at {} starts above the frame ending at {}",
        rule.y,
        frame.bottom()
    );
    let mut lines: Vec<Rect> = rects(&ui, "FieldLine");
    lines.sort_by(|a, b| a.y.total_cmp(&b.y));
    let first = lines.first().expect("a field row");
    assert!(
        first.y >= rule.bottom() - SLACK,
        "the first row at {} starts above the rule ending at {}",
        first.y,
        rule.bottom()
    );
}

#[test]
fn the_character_page_holds_at_800() {
    let ui = cards_ui(800.0, 600.0);
    ui.navigate(EditorCommand::Party, 0);
    ui.commit(ui.level().key, "34");
    assert!(ui.bridge().get_has_notice());
    assert_eq!(ui.bridge().get_pending_count(), 1);

    let window = Rect {
        x: 0.0,
        y: 0.0,
        w: 800.0,
        h: 600.0,
    };
    let page = the_rect(&ui, "CharacterEditPage");
    assert!(window.holds(&page), "the page ({page:?}) leaves the window");
    let list = the_rect(&ui, "FieldList");
    let shown = rects(&ui, "FieldLine").iter().filter(|line| list.holds(line)).count();
    assert!(shown >= 6, "{shown} rows fit whole in the list");

    let frame = the_rect(&ui, "PortraitFrame");
    let bar = the_rect(&ui, "PendingBar");
    assert!(
        bar.y >= frame.bottom() - SLACK,
        "the bar at {} starts above the frame ending at {}",
        bar.y,
        frame.bottom()
    );
    assert!(
        bar.y >= page.bottom() - SLACK,
        "the bar at {} starts above the page ending at {}",
        bar.y,
        page.bottom()
    );
}

#[test]
fn a_character_without_a_portrait_keeps_the_68_px_frame() {
    let ui = cards_ui(1280.0, 800.0);
    ui.app.editor.set_assets(Arc::new(ProviderChain::new(Vec::new())));
    refresh_editor(&ui.window, &ui.app);
    ui.navigate(EditorCommand::Party, 0);

    assert!(sprite_sizes(&ui).is_empty(), "no sprite is drawn");
    let frame = the_rect(&ui, "PortraitFrame");
    assert!((frame.w - 68.0).abs() <= SLACK && (frame.h - 68.0).abs() <= SLACK, "the frame is {frame:?}");
    let frame_element = the_element(&ui, "PortraitFrame");
    let initial = texts_of(&frame_element).into_iter().find(|(text, _)| text == "A").expect("the initial A");
    assert!(frame.holds(&initial.1), "the initial ({:?}) leaves the frame ({frame:?})", initial.1);
}
