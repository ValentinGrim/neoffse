//! The process names the running-game check looks for.

use neoffse_format_pr::games::GAMES;

/// The executable names of the game whose `GAMES` id is `game_id`: one for a
/// Pixel Remaster game, none for an id with no row.
pub(crate) fn executables_of(game_id: &str) -> Vec<&'static str> {
    GAMES.iter().filter(|row| row.id == game_id).map(|row| row.executable).collect()
}
