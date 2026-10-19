//! The Windows search: the Documents folder and its OneDrive variants.

use std::fs;
use std::path::PathBuf;

use super::{Discovery, Environment, Origin, SteamGame, probe, settle};
use crate::error::Operation;

/// Probes each distinct Documents folder for each game.
///
/// Source: `docs/formats/pixel-remaster.md#save-folder`. Steam roots and
/// libraries play no part here.
pub(super) fn search(environment: &Environment, games: &[SteamGame<'_>], found: &mut Discovery) {
    // The known folder first, then the fallbacks of the Go editor. On a
    // OneDrive machine the known folder and a fallback name one folder.
    let candidates = [
        environment.documents.clone(),
        environment.user_profile.as_ref().map(|profile| profile.join("Documents")),
        environment.user_profile.as_ref().map(|profile| profile.join("OneDrive").join("Documents")),
        environment.one_drive.as_ref().map(|drive| drive.join("Documents")),
    ];
    let mut keys: Vec<PathBuf> = Vec::new();
    for candidate in candidates.into_iter().flatten() {
        let Some(key) = settle(fs::canonicalize(&candidate), Operation::Read, &candidate, &mut found.skipped) else {
            continue;
        };
        if keys.contains(&key) {
            continue;
        }
        keys.push(key);
        // The result keeps the candidate's own spelling: `canonicalize` adds
        // a `\\?\` prefix on Windows, which a caller should not display.
        let origin = Origin::Documents { documents: candidate.clone() };
        for game in games {
            let steam_dir = candidate.join("My Games").join(game.folder).join("Steam");
            probe(&steam_dir, game, &origin, found);
        }
    }
}
