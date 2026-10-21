//! `games::identify` and the game rule of `Ff1Format::open`
//! (`docs/formats/pixel-remaster.md#telling-the-games-apart`), on the
//! synthetic FF I slot with its transportation list resized and marker keys
//! added.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::embed;
use common::ff1_format::{one_file, open, sav, slot_name, standard_files};
use common::ff1_save::SlotData;
use neoffse_core::{FormatError, Game};
use neoffse_format_pr::container::Container;
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff1::{Ff1Format, OpenError};
use neoffse_format_pr::games::identify;

const TARGET: [Segment<'static>; 5] = [
    Segment::Key("userData"),
    Segment::Doc,
    Segment::Key("ownedTransportationList"),
    Segment::Doc,
    Segment::Key("target"),
];
const VEHICLE: &str = r#"{"position":{"x":1000.0,"y":1000.0,"z":-1000.0},"direction":0,"id":1,"mapId":-1,"enable":false,"timeStampTicks":0}"#;

/// The standard slot text with `target` holding `count` entries.
fn with_vehicles(count: usize) -> String {
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    for _ in 4..count {
        document.insert_raw(&TARGET, 0, &embed(VEHICLE)).unwrap();
    }
    for _ in count..4 {
        document.remove(&TARGET, 0).unwrap();
    }
    document.to_json()
}

/// `text` with the first `from` replaced by `to`; the replace must hit.
fn replaced(text: &str, from: &str, to: &str) -> String {
    let result = text.replacen(from, to, 1);
    assert_ne!(result, text, "{from} is not in the slot text");
    result
}

fn open_text(text: &str) -> Result<neoffse_format_pr::ff1::Ff1Document, FormatError> {
    let files = one_file(slot_name(1), sav(text, &Container::game()));
    Ff1Format::new().open_document(&files)
}

fn open_error(result: Result<neoffse_format_pr::ff1::Ff1Document, FormatError>) -> OpenError {
    let Err(FormatError::Decode { source }) = result else {
        panic!("expected a Decode error")
    };
    *source.downcast::<OpenError>().expect("the source is an OpenError")
}

fn identified(text: &str) -> Result<Game, OpenError> {
    identify(&mut Document::parse(text).unwrap())
}

#[test]
fn the_standard_slot_is_ff1() {
    assert_eq!(identified(&SlotData::standard().render()).unwrap(), Game::Ff1Pr);
    assert_eq!(open(&standard_files(&Container::game())).file_name(), slot_name(1));
}

#[test]
fn a_slot_of_another_game_is_refused() {
    let games = [(7, Game::Ff2Pr), (14, Game::Ff3Pr), (11, Game::Ff4Pr), (19, Game::Ff5Pr), (8, Game::Ff6Pr)];
    for (count, game) in games {
        let text = with_vehicles(count);
        assert_eq!(identified(&text).unwrap(), game, "{count} vehicles");
        match open_error(open_text(&text)) {
            OpenError::OtherGame { found, vehicles } => assert_eq!((found, vehicles), (Some(game), count)),
            other => panic!("{count} vehicles: {other:?}"),
        }
    }
}

#[test]
fn a_count_no_game_has_names_no_game() {
    for count in [0, 5, 20] {
        let text = with_vehicles(count);
        assert!(matches!(identified(&text), Err(OpenError::OtherGame { found: None, vehicles }) if vehicles == count));
        assert!(matches!(open_error(open_text(&text)), OpenError::OtherGame { found: None, vehicles } if vehicles == count));
    }
}

#[test]
fn a_marker_key_of_another_game_refuses_a_count_of_four() {
    let slot = SlotData::standard().render();
    let cases = [
        (r#"\"cheatSettingsData\":{"#, r#"\"cheatSettingsData\":{\"jobProficiencyRate\":1,"#, Game::Ff3Pr),
        (
            r#"\"isOpenedGameBoosterWindow\""#,
            r#"\"wonderWandIndex\":0,\"isOpenedGameBoosterWindow\""#,
            Game::Ff5Pr,
        ),
        (r#"\"moveCount\":10"#, r#"\"otherPartyDataList\":[],\"moveCount\":10"#, Game::Ff6Pr),
    ];
    for (from, to, game) in cases {
        let text = replaced(&slot, from, to);
        assert!(
            matches!(identified(&text), Err(OpenError::OtherGame { found: Some(found), vehicles: 4 }) if found == game),
            "{game:?}"
        );
        assert!(
            matches!(open_error(open_text(&text)), OpenError::OtherGame { found: Some(found), vehicles: 4 } if found == game),
            "{game:?}"
        );
    }
}

#[test]
fn a_game_keeps_its_own_marker() {
    let text = replaced(
        &with_vehicles(14),
        r#"\"cheatSettingsData\":{"#,
        r#"\"cheatSettingsData\":{\"jobProficiencyRate\":1,"#,
    );
    assert_eq!(identified(&text).unwrap(), Game::Ff3Pr);
}

#[test]
fn a_missing_list_or_a_target_that_is_no_array_is_not_a_slot() {
    let slot = SlotData::standard().render();
    let missing = replaced(&slot, r#"\"ownedTransportationList\""#, r#"\"ownedTransportationListX\""#);
    let mut flat = Document::parse(&slot).unwrap();
    flat.set_raw(&TARGET, "{}").unwrap();
    let mut text = Document::parse(&slot).unwrap();
    text.set(&TARGET, &Scalar::Text("x".to_owned())).unwrap();
    for text in [missing, flat.to_json(), text.to_json()] {
        assert!(matches!(identified(&text), Err(OpenError::NotASlot { .. })));
        assert!(matches!(open_error(open_text(&text)), OpenError::NotASlot { .. }));
    }
}

#[test]
fn a_document_without_user_data_is_not_a_slot() {
    let mut slot = SlotData::standard();
    slot.with_user_data = false;
    assert!(matches!(identified(&slot.render()), Err(OpenError::NotASlot { .. })));
}

#[test]
fn identify_leaves_a_spaced_document_byte_for_byte() {
    let mut slot = SlotData::standard();
    slot.spaced = true;
    let text = slot.render();
    let mut document = Document::parse(&text).unwrap();
    assert_eq!(identify(&mut document).unwrap(), Game::Ff1Pr);
    assert_eq!(document.to_json(), text);
    let refused = replaced(&text, r#"\"moveCount\":10"#, r#"\"otherPartyDataList\":[],\"moveCount\":10"#);
    let mut document = Document::parse(&refused).unwrap();
    assert!(identify(&mut document).is_err());
    assert_eq!(document.to_json(), refused);
}

const FF6_MARKER: (&str, &str) = (r#"\"moveCount\":10"#, r#"\"otherPartyDataList\":[],\"moveCount\":10"#);

#[test]
fn open_names_the_game_of_the_count_before_any_marker() {
    let text = replaced(&with_vehicles(7), FF6_MARKER.0, FF6_MARKER.1);
    match open_error(open_text(&text)) {
        OpenError::OtherGame { found, vehicles } => assert_eq!((found, vehicles), (Some(Game::Ff2Pr), 7)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn identify_names_the_marker_game_when_the_marker_contradicts_the_count() {
    let text = replaced(&with_vehicles(7), FF6_MARKER.0, FF6_MARKER.1);
    assert!(matches!(
        identified(&text),
        Err(OpenError::OtherGame {
            found: Some(Game::Ff6Pr),
            vehicles: 7
        })
    ));
}

#[test]
fn a_marker_path_that_crosses_a_value_of_the_wrong_kind_is_not_a_slot() {
    let text = replaced(
        &SlotData::standard().render(),
        r#"\"cheatSettingsData\":{"#,
        r#"\"cheatSettingsData\":1,\"x\":{"#,
    );
    assert!(matches!(identified(&text), Err(OpenError::NotASlot { .. })));
    assert!(matches!(open_error(open_text(&text)), OpenError::NotASlot { .. }));
}
