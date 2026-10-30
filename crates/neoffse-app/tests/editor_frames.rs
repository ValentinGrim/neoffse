//! The banded frame of the Pixel Remaster editor: six bands on a panel, the
//! two-line divider, the two lines of the portrait box. Home and the Shell
//! dialogs keep the flat frame. Geometry comes from the accessibility tree of
//! Slint's testing backend; no window shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Rect, SLACK, Ui, bands_of, editing_site, first_element, open_editor, resize};
use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use neoffse_app::{EditorCommand, FrameStyle, Theme};
use slint::ComponentHandle;

/// The bands of the first frame under `element`: the panel's own six, not
/// those of a control inside it.
fn own_bands(element: &ElementHandle) -> Vec<Rect> {
    let frame = element
        .query_descendants()
        .match_type_name("ControlFrame")
        .find_first()
        .expect("a panel draws a frame");
    bands_of(&frame)
}

fn all_of(ui: &Ui, type_name: &str) -> Vec<ElementHandle> {
    ElementHandle::find_by_element_type_name(&ui.window, type_name).collect()
}

fn band_count(ui: &Ui) -> usize {
    ElementHandle::find_by_element_type_name(&ui.window, "ControlFrame")
        .map(|frame| bands_of(&frame).len())
        .sum()
}

fn on_party_cards() -> Ui {
    let site = editing_site();
    let ui = open_editor(&site);
    resize(&ui, 1280.0, 800.0);
    ui.navigate(EditorCommand::Party, -1);
    ui
}

fn assert_six_apart(what: &str, panel: &Rect, bands: &[Rect]) {
    assert_eq!(bands.len(), 6, "{what} draws {} bands", bands.len());
    for (k, band) in (0_u8..).map(f32::from).zip(bands) {
        let expected = Rect {
            x: panel.x + k,
            y: panel.y + k,
            w: panel.w - 2.0 * k,
            h: panel.h - 2.0 * k,
        };
        assert!(
            (band.x - expected.x).abs() <= SLACK
                && (band.y - expected.y).abs() <= SLACK
                && (band.w - expected.w).abs() <= SLACK
                && (band.h - expected.h).abs() <= SLACK,
            "{what}: band {k} is {band:?}, expected {expected:?}"
        );
    }
}

#[test]
fn a_panel_draws_six_bands_one_pixel_apart() {
    let ui = on_party_cards();
    let list = first_element(&ui, "CommandList");
    assert_six_apart("the command list", &Rect::of(&list), &bands_of(&list));
}

#[test]
fn the_bands_follow_the_frame_width() {
    let ui = on_party_cards();
    let look = ui.window.global::<Theme>().get_look();
    assert!((look.frame_width - 1.0).abs() < f32::EPSILON);
    assert!((look.frame_radius - 6.0).abs() < f32::EPSILON);
    assert_eq!(look.frame_style, FrameStyle::Bevel);

    let bands = bands_of(&first_element(&ui, "CommandList"));
    for pair in bands.windows(2) {
        let step = (pair[0].w - pair[1].w) / 2.0;
        assert!((step - look.frame_width).abs() <= SLACK, "a step of {step} px between two bands");
    }
}

#[test]
fn every_panel_is_banded() {
    let ui = on_party_cards();
    let panels = all_of(&ui, "Frame");
    assert!(panels.len() >= 3, "the lists, the status panel and the page are frames");
    for panel in &panels {
        assert_six_apart("a panel", &Rect::of(panel), &own_bands(panel));
    }
    let status = first_element(&ui, "StatusPanel");
    assert_six_apart("the status panel", &Rect::of(&status), &own_bands(&status));
}

#[test]
fn a_rule_is_two_pixels_high_in_the_game_look() {
    let ui = on_party_cards();
    let width = ui.window.global::<Theme>().get_look().frame_width;
    let rules = all_of(&ui, "RuleLine");
    assert!(!rules.is_empty(), "the status panel and the cards hold dividers");
    for rule in &rules {
        let height = Rect::of(rule).h;
        assert!((height - 2.0 * width).abs() <= SLACK, "a rule is {height} px high");
    }
}

#[test]
fn the_portrait_frame_draws_two_lines() {
    let ui = on_party_cards();
    let frames = all_of(&ui, "PortraitFrame");
    assert!(!frames.is_empty(), "the cards show portrait frames");
    for frame in &frames {
        let outer = Rect::of(frame);
        let edge = frame.query_descendants().match_id("PortraitFrame::edge").find_first().expect("an edge");
        let inner = frame
            .query_descendants()
            .match_id("PortraitFrame::inner-edge")
            .find_first()
            .expect("an inner edge");
        let (edge, inner) = (Rect::of(&edge), Rect::of(&inner));
        assert!((edge.w - outer.w).abs() <= SLACK && (edge.h - outer.h).abs() <= SLACK);
        assert!((inner.x - edge.x - 1.0).abs() <= SLACK && (inner.y - edge.y - 1.0).abs() <= SLACK);
        assert!((inner.w - (outer.w - 2.0)).abs() <= SLACK && (inner.h - (outer.h - 2.0)).abs() <= SLACK);
        assert!((outer.w - 68.0).abs() <= SLACK && (outer.h - 68.0).abs() <= SLACK, "the frame is {outer:?}");
        if let Some(sprite) = frame.query_descendants().match_id("PortraitFrame::sprite").find_first() {
            let sprite = Rect::of(&sprite);
            assert!(
                (sprite.x - outer.x - 2.0).abs() <= SLACK && (sprite.y - outer.y - 2.0).abs() <= SLACK,
                "the sprite is at {sprite:?}"
            );
        }
    }
    let sprites = ElementHandle::find_by_element_type_name(&ui.window, "PortraitFrame")
        .filter_map(|frame| frame.query_descendants().match_id("PortraitFrame::sprite").find_first())
        .map(|sprite| (sprite.size().width, sprite.size().height))
        .collect::<Vec<_>>();
    assert!(!sprites.is_empty(), "the fixture's jobs have sprites");
    assert!(sprites.iter().all(|&size| size == (64.0, 64.0)), "sprites: {sprites:?}");
}

#[test]
fn the_lines_of_the_portrait_frame_never_cover_the_sprite() {
    let ui = on_party_cards();
    let mut checked = 0;
    for frame in &all_of(&ui, "PortraitFrame") {
        let Some(sprite) = frame.query_descendants().match_id("PortraitFrame::sprite").find_first() else {
            continue;
        };
        let inner = frame
            .query_descendants()
            .match_id("PortraitFrame::inner-edge")
            .find_first()
            .expect("an inner edge");
        let (inner, sprite) = (Rect::of(&inner), Rect::of(&sprite));
        let free = Rect {
            x: inner.x + 1.0,
            y: inner.y + 1.0,
            w: inner.w - 2.0,
            h: inner.h - 2.0,
        };
        assert!(free.holds(&sprite), "the sprite {sprite:?} reaches the line of {inner:?}");
        checked += 1;
    }
    assert!(checked > 0, "the fixture's jobs have sprites");
}

#[test]
fn home_draws_no_band() {
    let site = editing_site();
    let ui = Ui::scanned(&site, None);
    resize(&ui, 800.0, 600.0);
    assert_eq!(ui.window.global::<Theme>().get_look().frame_style, FrameStyle::Flat);
    assert!(
        ElementHandle::find_by_element_type_name(&ui.window, "ControlFrame").count() > 0,
        "Home draws control frames"
    );
    assert_eq!(band_count(&ui), 0, "Home draws a band");
}

#[test]
fn a_shell_dialog_inside_the_pr_editor_stays_flat() {
    let site = editing_site();
    let ui = open_editor(&site);
    resize(&ui, 1280.0, 800.0);
    ui.navigate(EditorCommand::Party, -1);
    ui.make_dirty();
    ui.bridge().invoke_review_requested();
    assert!(ui.bridge().get_review_open());

    let save = Rect::of(&ui.element("Save", AccessibleRole::Button));
    let button = ElementHandle::find_by_element_type_name(&ui.window, "ThemedButton")
        .filter(|button| Rect::of(button).holds(&save))
        .min_by(|a, b| Rect::of(a).w.total_cmp(&Rect::of(b).w))
        .expect("the dialog's Save button");
    assert!(bands_of(&button).is_empty(), "the dialog button draws a band");
    assert_six_apart(
        "the command list",
        &Rect::of(&first_element(&ui, "CommandList")),
        &bands_of(&first_element(&ui, "CommandList")),
    );
}
