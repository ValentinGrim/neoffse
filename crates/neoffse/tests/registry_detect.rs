//! `Registry::detect`: the claim of a file, with and without the folder's game.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{ff1_registry as registry, slot_bytes, slot_name};
use neoffse::core::{Confidence, FileProbe, Game, SaveSlot};

fn probe_of<'a>(name: &'a str, bytes: &'a [u8]) -> FileProbe<'a> {
    FileProbe::new(name, bytes.len() as u64, bytes)
}

#[test]
fn a_slot_name_with_a_base64_head_is_likely_ff1() {
    let bytes = slot_bytes();
    let probe = probe_of(slot_name(2), &bytes);
    for folder_game in [None, Some(Game::Ff1Pr)] {
        let claim = registry().detect(&probe, folder_game).unwrap();
        assert_eq!(claim.game, Game::Ff1Pr);
        assert_eq!(claim.detection.confidence, Confidence::Likely);
        assert_eq!(claim.detection.slot, Some(SaveSlot::Numbered(1)));
    }
}

#[test]
fn another_folder_game_gets_no_claim_while_the_registry_lacks_its_format() {
    let bytes = slot_bytes();
    let probe = probe_of(slot_name(2), &bytes);
    assert_eq!(registry().detect(&probe, Some(Game::Ff2Pr)), None);
}

#[test]
fn a_temp_name_is_never_claimed() {
    let bytes = slot_bytes();
    let probe = probe_of(".neoffse-abc.tmp", &bytes);
    assert_eq!(registry().detect(&probe, None), None);
    assert_eq!(registry().detect(&probe, Some(Game::Ff1Pr)), None);
}

#[test]
fn an_unrelated_name_is_not_claimed() {
    let probe = probe_of("steam_autocloud.vdf", b"\"AutoCloud\"");
    assert_eq!(registry().detect(&probe, None), None);
    assert_eq!(registry().detect(&probe, Some(Game::Ff1Pr)), None);
}

#[test]
fn a_slot_name_without_a_base64_head_is_possible() {
    let probe = probe_of(slot_name(0), b"{\"not\": \"base64\"}");
    let claim = registry().detect(&probe, None).unwrap();
    assert_eq!(claim.detection.confidence, Confidence::Possible);
    assert_eq!(claim.detection.slot, Some(SaveSlot::Auto));
}
