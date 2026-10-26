//! The theme follows the open save's game. Bridge and globals only: no
//! window shows and no event loop runs.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{ID_1, Site, Ui, editing_site, open_editor, slot_file, steam_slot};
use neoffse::core::Game;
use neoffse_app::state::AppState;
use neoffse_app::{Fonts, FrameStyle, Shell, Theme, ThemeId, theme_for};
use slint::{Brush, ComponentHandle};

fn id(ui: &Ui) -> ThemeId {
    ui.window.global::<Theme>().get_id()
}

#[test]
fn every_known_game_has_a_theme() {
    for &game in Game::ALL {
        assert_ne!(theme_for(game), ThemeId::App, "{} has no theme of its own", game.id());
    }
}

#[test]
fn home_starts_in_the_app_theme() {
    let site = Site::new();
    let ui = Ui::new(&site, None);
    let theme = ui.window.global::<Theme>();
    assert_eq!(theme.get_id(), ThemeId::App);
    assert_eq!(theme.get_look(), ui.window.global::<Shell>().get_look());
    let look = theme.get_look();
    assert_eq!(look.font_family, ui.window.global::<Fonts>().get_modern());
    assert_eq!(look.frame_style, FrameStyle::Flat);
}

#[test]
fn opening_a_save_selects_the_pr_theme() {
    let site = editing_site();
    let ui = open_editor(&site);
    let theme = ui.window.global::<Theme>();
    assert_eq!(theme.get_id(), ThemeId::Pr);
    let look = theme.get_look();
    assert_eq!(look.font_family.as_str(), "Cornelia Sans");
    assert_eq!(look.font_family, ui.window.global::<Fonts>().get_pixel());
    assert_eq!(look.frame_style, FrameStyle::Bevel);
    assert_ne!(look, ui.window.global::<Shell>().get_look());
}

#[test]
fn back_returns_to_the_app_theme() {
    let site = editing_site();
    let ui = open_editor(&site);
    assert_eq!(id(&ui), ThemeId::Pr);
    ui.window.invoke_back_requested();
    assert!(!ui.window.get_editor_open());
    assert_eq!(id(&ui), ThemeId::App);

    ui.window.invoke_slot_clicked(ui.first_slots()[0].key);
    ui.manual.run_all();
    assert_eq!(id(&ui), ThemeId::Pr);
    ui.commit(ui.gil().key, "500");
    ui.window.invoke_back_requested();
    assert!(ui.window.get_editor_open(), "a dirty save asks first");
    assert_eq!(id(&ui), ThemeId::Pr, "the question keeps the game's look");
    ui.bridge().invoke_discard_confirmed();
    assert!(!ui.window.get_editor_open());
    assert_eq!(id(&ui), ThemeId::App);
}

#[test]
fn a_failed_open_leaves_the_app_theme() {
    let site = Site::new();
    steam_slot(&site, 0, ID_1, slot_file(1), b"not a save");
    let ui = Ui::scanned(&site, None);
    ui.window.invoke_slot_clicked(ui.first_slots()[0].key);
    ui.manual.run_all();
    assert!(ui.window.get_has_open_problem(), "the open fails");
    assert_eq!(id(&ui), ThemeId::App);
}

#[test]
fn the_game_accessor_follows_the_state() {
    let site = editing_site();
    let ui = open_editor(&site);
    let state: &AppState = &ui.app.state;
    assert_eq!(state.game(), Some(Game::Ff1Pr));
    state.close();
    assert_eq!(state.game(), None);
    assert_eq!(AppState::new().game(), None);
}

#[test]
fn the_shell_look_is_flat_and_the_palette_is_navy() {
    let site = Site::new();
    let ui = Ui::new(&site, None);
    let shell = ui.window.global::<Shell>().get_look();
    assert_eq!(shell.frame_style, FrameStyle::Flat);
    assert_eq!(shell.frame_width, 1.0);
    // The Pixel Remaster look draws a flat colour here: the shell differs.
    let pr_fill = Brush::SolidColor(slint::Color::from_rgb_u8(0x05, 0x07, 0x2a));
    assert_ne!(shell.window_fill, pr_fill);
    assert!(matches!(shell.window_fill, Brush::LinearGradient(_)), "the shell window is a navy gradient");
}

#[test]
fn the_pr_font_sizes_sit_on_the_pixel_grid() {
    let site = editing_site();
    let ui = open_editor(&site);
    let look = ui.window.global::<Theme>().get_look();
    // Cornelia Sans draws even strokes at multiples of 12 px only.
    for (name, size) in [("body", look.font_body), ("small", look.font_small), ("title", look.font_title)] {
        assert!((size % 12.0).abs() < f32::EPSILON, "the {name} size {size} is off the 12 px grid");
    }
    assert!(look.font_body >= look.font_small && look.font_title > look.font_body);
}

#[test]
fn the_pr_text_is_as_dense_as_home() {
    let site = editing_site();
    let ui = open_editor(&site);
    let pr = ui.window.global::<Theme>().get_look();
    assert_eq!((pr.font_body, pr.font_small, pr.font_title, pr.row_height), (12.0, 12.0, 24.0, 30.0));
    // The Shell numbers pin that Home does not move.
    let shell = ui.window.global::<Shell>().get_look();
    assert_eq!(
        (shell.font_body, shell.font_small, shell.font_title, shell.row_height),
        (14.0, 12.0, 18.0, 30.0)
    );
}

#[test]
fn the_pr_body_is_no_larger_than_twice_home_body() {
    let site = editing_site();
    let ui = open_editor(&site);
    let pr = ui.window.global::<Theme>().get_look();
    let shell = ui.window.global::<Shell>().get_look();
    // A ceiling, not a pin: a return to 24 px text fails it.
    assert!(
        pr.font_body <= 2.0 * shell.font_body,
        "PR body {} against Home body {}",
        pr.font_body,
        shell.font_body
    );
}
