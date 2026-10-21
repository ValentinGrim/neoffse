//! Picks the look of the window from the open save's game.
//!
//! The `.slint` side holds every colour, size and font (`ui/theme.slint`,
//! `ui/shell.slint`). This module only maps a game to the theme that draws it.

use neoffse::core::Game;
use slint::ComponentHandle;

use crate::{MainWindow, Theme, ThemeId};

/// The theme a game uses. Every Pixel Remaster game shares `Pr`; a game with
/// no theme of its own gets `App`.
#[must_use]
pub fn theme_for(game: Game) -> ThemeId {
    match game {
        Game::Ff1Pr | Game::Ff2Pr | Game::Ff3Pr | Game::Ff4Pr | Game::Ff5Pr | Game::Ff6Pr => ThemeId::Pr,
        _ => ThemeId::App,
    }
}

/// Sets `Theme.id` from `game`; `None` selects the app theme.
pub fn sync_theme(window: &MainWindow, game: Option<Game>) {
    let id = game.map_or(ThemeId::App, theme_for);
    window.global::<Theme>().set_id(id);
}
