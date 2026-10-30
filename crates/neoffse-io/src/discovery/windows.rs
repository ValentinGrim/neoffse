//! The Windows search: the Documents folder and its OneDrive variants for
//! saves, the Steam libraries under `Program Files (x86)` for installs.

use std::fs;
use std::path::PathBuf;

use super::steam::{installs, libraries, roots};
use super::{Discovery, Environment, Origin, SteamGame, probe, settle};
use crate::error::Operation;

/// Probes each distinct Documents folder for each game, then lists the
/// installs of the Steam libraries.
///
/// Source: `docs/formats/pixel-remaster.md#save-folder`. Saves ignore Steam
/// libraries; the libraries add installs only.
pub(super) fn search(environment: &Environment, games: &[SteamGame<'_>], found: &mut Discovery) {
    save_folders(environment, games, found);
    steam_installs(environment, games, found);
}

fn save_folders(environment: &Environment, games: &[SteamGame<'_>], found: &mut Discovery) {
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

/// The root is `<program_files_x86>/Steam`, read from the environment and not
/// from the registry. It counts when it holds a `steamapps` folder.
fn steam_installs(environment: &Environment, games: &[SteamGame<'_>], found: &mut Discovery) {
    let Some(program_files) = environment.program_files_x86.as_deref() else {
        return;
    };
    let roots = roots(&[program_files.join("Steam")], &mut found.skipped);
    let libraries = libraries(roots, &mut found.skipped);
    installs(&libraries, games, found);
}
