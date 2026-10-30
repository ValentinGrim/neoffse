//! The process names the running-game check looks for.

use neoffse_format_pr::games::GAMES;

use crate::error::Error;
use crate::session::Running;

/// The executable names of the game whose `GAMES` id is `game_id`: one for a
/// Pixel Remaster game, none for an id with no row.
fn executables_of(game_id: &str) -> Vec<&'static str> {
    GAMES.iter().filter(|row| row.id == game_id).map(|row| row.executable).collect()
}

/// Refuses while the game's process runs: with [`Running::Refuse`], scans for
/// the executables of `game_id` through `probe` and fails with
/// [`Error::GameRunning`] naming those found.
///
/// [`Running::Allow`] skips the scan, and so does a game with no executable
/// row: the probe is never called in either case.
pub(crate) fn refuse_if_running<P>(game_id: &str, running: Running, probe: P) -> Result<(), Error>
where
    P: Fn(&[&'static str]) -> Vec<&'static str>,
{
    if running != Running::Refuse {
        return Ok(());
    }
    let executables = executables_of(game_id);
    if executables.is_empty() {
        return Ok(());
    }
    let found = probe(&executables);
    if found.is_empty() {
        Ok(())
    } else {
        Err(Error::GameRunning { executables: found })
    }
}
