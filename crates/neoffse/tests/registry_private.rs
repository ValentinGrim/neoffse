//! The registry on a copy of the owner's FF I saves (`NEOFFSE_SAVES`, read
//! only). The tests skip when the folder is absent and print slot labels,
//! never file names.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::Registry;
use neoffse::core::Game;
use neoffse::data::Language;
use neoffse::pr::slots::SLOT_FILES;

#[test]
fn every_private_slot_file_is_claimed_and_opens() {
    let Some(paths) = neoffse_testkit::private_saves("ff1") else { return };
    let Some(dir) = paths.first().and_then(|path| path.parent()) else { return };
    let registry = Registry::new(Language::En);

    let scan = registry.scan(dir, Some(Game::Ff1Pr)).unwrap();
    let slot_files = paths
        .iter()
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| SLOT_FILES.iter().any(|file| file.name == name))
        })
        .count();
    assert_eq!(scan.saves.len(), slot_files, "the scan must claim every slot file");
    assert!(scan.unreadable.is_empty());

    for found in &scan.saves {
        let slot = found.claim.detection.slot;
        for game in [Some(Game::Ff1Pr), None] {
            let save = registry.open(dir, &found.name, game);
            assert!(save.is_ok(), "slot {slot:?} must open with game {game:?}");
        }
    }
}
