//! `Registry::detect`, `scan` and `open` with several formats: the tie-break,
//! the confidence, the folder game, the temp-name guard and the first `Ok`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Refusal, StandIn, folder_with, must_some};
use neoffse::core::{Confidence, FileProbe, FormatError, Game};
use neoffse::{Error, Registry};

const NAME: &str = "any.bin";

/// A registry of stand-ins that claim every name, in the order given.
fn claiming(claims: &[(Game, Confidence)]) -> Registry {
    Registry::from_formats(
        claims
            .iter()
            .map(|(game, confidence)| StandIn::new(*game).claiming(*confidence).arc())
            .collect(),
    )
}

fn probe(name: &str) -> FileProbe<'_> {
    FileProbe::new(name, 1, b"x")
}

fn game_of(registry: &Registry, folder_game: Option<Game>) -> Game {
    must_some(registry.detect(&probe(NAME), folder_game), "a format claims the file").game
}

#[test]
fn equal_claims_go_to_the_first_game_of_all() {
    let registry = claiming(&[(Game::Ff3Pr, Confidence::Likely), (Game::Ff1Pr, Confidence::Likely)]);
    assert_eq!(game_of(&registry, None), Game::Ff1Pr);
    let registry = claiming(&[
        (Game::Ff6Pr, Confidence::Likely),
        (Game::Ff2Pr, Confidence::Likely),
        (Game::Ff5Pr, Confidence::Likely),
    ]);
    assert_eq!(game_of(&registry, None), Game::Ff2Pr);
}

#[test]
fn the_higher_confidence_wins_over_an_earlier_game() {
    let registry = claiming(&[
        (Game::Ff1Pr, Confidence::Possible),
        (Game::Ff3Pr, Confidence::Likely),
        (Game::Ff5Pr, Confidence::Certain),
    ]);
    let claim = must_some(registry.detect(&probe(NAME), None), "a format claims the file");
    assert_eq!((claim.game, claim.detection.confidence), (Game::Ff5Pr, Confidence::Certain));
    let registry = claiming(&[(Game::Ff1Pr, Confidence::Likely), (Game::Ff3Pr, Confidence::Possible)]);
    assert_eq!(game_of(&registry, None), Game::Ff1Pr);
}

#[test]
fn a_folder_game_decides_against_a_surer_claim() {
    let registry = claiming(&[(Game::Ff1Pr, Confidence::Possible), (Game::Ff3Pr, Confidence::Certain)]);
    let claim = must_some(registry.detect(&probe(NAME), Some(Game::Ff1Pr)), "the folder game claims the file");
    assert_eq!((claim.game, claim.detection.confidence), (Game::Ff1Pr, Confidence::Possible));
}

#[test]
fn a_folder_game_whose_format_declines_gets_none() {
    let registry = Registry::from_formats(vec![
        StandIn::new(Game::Ff1Pr).arc(),
        StandIn::new(Game::Ff3Pr).claiming(Confidence::Certain).arc(),
    ]);
    assert_eq!(registry.detect(&probe(NAME), Some(Game::Ff1Pr)), None);
    assert_eq!(game_of(&registry, None), Game::Ff3Pr);
}

#[test]
fn a_folder_game_without_a_format_gets_none() {
    let registry = claiming(&[(Game::Ff1Pr, Confidence::Certain)]);
    assert_eq!(registry.detect(&probe(NAME), Some(Game::Ff4Pr)), None);
}

#[test]
fn no_claim_gives_none() {
    let registry = Registry::from_formats(vec![StandIn::new(Game::Ff1Pr).arc(), StandIn::new(Game::Ff2Pr).arc()]);
    assert_eq!(registry.detect(&probe(NAME), None), None);
    assert_eq!(registry.detect(&probe(NAME), Some(Game::Ff1Pr)), None);
    assert_eq!(registry.detect(&probe(NAME), Some(Game::Ff2Pr)), None);
}

#[test]
fn a_temp_name_is_never_claimed_by_a_format_that_claims_everything() {
    let registry = claiming(&[(Game::Ff1Pr, Confidence::Certain)]);
    assert_eq!(registry.detect(&probe(".neoffse-abc.tmp"), None), None);
    assert_eq!(registry.detect(&probe(".neoffse-abc.tmp"), Some(Game::Ff1Pr)), None);
    assert_eq!(game_of(&registry, None), Game::Ff1Pr);
}

#[test]
fn a_scan_ranks_each_file_like_detect() {
    let registry = claiming(&[(Game::Ff1Pr, Confidence::Possible), (Game::Ff3Pr, Confidence::Certain)]);
    let dir = folder_with(&[("a.bin", b"x"), ("b.bin", b"x"), (".neoffse-x.tmp", b"x")]);
    for (folder_game, expected) in [(None, Game::Ff3Pr), (Some(Game::Ff1Pr), Game::Ff1Pr)] {
        let scan = registry.scan(dir.path(), folder_game).unwrap();
        let found: Vec<(&str, Game)> = scan.saves.iter().map(|file| (file.name.as_str(), file.claim.game)).collect();
        assert_eq!(found, vec![("a.bin", expected), ("b.bin", expected)]);
    }
}

fn registry_of(formats: Vec<StandIn>) -> Registry {
    Registry::from_formats(formats.into_iter().map(StandIn::arc).collect())
}

fn refusing(game: Game) -> StandIn {
    StandIn::new(game).refusing(Refusal::Garbage)
}

#[test]
fn without_a_game_the_first_game_of_all_that_opens_wins() {
    let registry = registry_of(vec![StandIn::new(Game::Ff3Pr), StandIn::new(Game::Ff1Pr)]);
    let dir = folder_with(&[(NAME, b"x")]);
    let mut session = registry.open(dir.path(), NAME, None).unwrap();
    assert_eq!(session.game(), Game::Ff1Pr);
    assert_eq!(session.summary().unwrap().game, Game::Ff1Pr);
}

#[test]
fn a_refusing_format_does_not_stop_the_next() {
    let registry = registry_of(vec![refusing(Game::Ff1Pr), refusing(Game::Ff2Pr), StandIn::new(Game::Ff3Pr)]);
    let dir = folder_with(&[(NAME, b"x")]);
    let session = registry.open(dir.path(), NAME, None).unwrap();
    assert_eq!(session.game(), Game::Ff3Pr);
}

#[test]
fn with_a_game_only_that_format_answers() {
    let registry = registry_of(vec![refusing(Game::Ff1Pr), StandIn::new(Game::Ff3Pr)]);
    let dir = folder_with(&[(NAME, b"x")]);
    let session = registry.open(dir.path(), NAME, Some(Game::Ff3Pr)).unwrap();
    assert_eq!(session.game(), Game::Ff3Pr);
    let error = registry.open(dir.path(), NAME, Some(Game::Ff1Pr)).unwrap_err();
    assert!(matches!(
        error,
        Error::Format {
            source: FormatError::Decode { .. }
        }
    ));
}

#[test]
fn when_none_opens_every_attempt_is_reported_in_all_order() {
    let registry = registry_of(vec![refusing(Game::Ff5Pr), refusing(Game::Ff1Pr), refusing(Game::Ff3Pr)]);
    let dir = folder_with(&[(NAME, b"x")]);
    let error = registry.open(dir.path(), NAME, None).unwrap_err();
    let Error::NoFormat { attempts } = error else { panic!("expected NoFormat") };
    let games: Vec<Game> = attempts.iter().map(|(game, _)| *game).collect();
    assert_eq!(games, vec![Game::Ff1Pr, Game::Ff3Pr, Game::Ff5Pr]);
    assert!(attempts.iter().all(|(_, error)| matches!(error, FormatError::Decode { .. })));
}

#[test]
fn an_empty_registry_reports_no_attempt() {
    let dir = folder_with(&[(NAME, b"x")]);
    let error = Registry::from_formats(Vec::new()).open(dir.path(), NAME, None).unwrap_err();
    let Error::NoFormat { attempts } = error else { panic!("expected NoFormat") };
    assert!(attempts.is_empty());
}

/// `open(None)` keeps the first `Ok` in `Game::ALL` order: a format that
/// claims the file with a higher confidence does not go first.
#[test]
fn open_does_not_rank_attempts_by_detect() {
    let registry = registry_of(vec![StandIn::new(Game::Ff1Pr), StandIn::new(Game::Ff3Pr).claiming(Confidence::Certain)]);
    let dir = folder_with(&[(NAME, b"x")]);
    let session = registry.open(dir.path(), NAME, None).unwrap();
    assert_eq!(session.game(), Game::Ff1Pr);
    assert_eq!(game_of(&registry, None), Game::Ff3Pr);
}
