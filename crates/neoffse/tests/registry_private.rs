//! The registry on a copy of the owner's FF I saves (`NEOFFSE_SAVES`, read
//! only). The tests skip when the folder holds no slot file and print slot
//! positions, never file names or values.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{must, must_some, private_slot_paths};
use neoffse::Registry;
use neoffse::core::Game;
use neoffse::data::Language;

#[test]
fn every_private_slot_file_is_claimed_and_opens() {
    let Some(slots) = private_slot_paths() else { return };
    let dir = must_some(slots.first().and_then(|(_, path)| path.parent()), "a slot file has a folder");
    let registry = Registry::new(Language::En);

    let scan = must(registry.scan(dir, Some(Game::Ff1Pr)), "scan the private folder");
    assert!(scan.saves.len() == slots.len(), "the scan must claim every slot file");
    assert!(scan.unreadable.is_empty());

    for (index, found) in scan.saves.iter().enumerate() {
        for game in [Some(Game::Ff1Pr), None] {
            let save = registry.open(dir, &found.name, game);
            assert!(save.is_ok(), "scanned slot {index} must open with game {game:?}");
        }
    }
}
