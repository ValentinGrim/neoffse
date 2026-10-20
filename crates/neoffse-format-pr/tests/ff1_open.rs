//! `Ff1Format::open_document` and the trait `open`: the container kept, the
//! shape gate and the sources of each error.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::embed;
use common::ff1_format::{one_file, open, sav, slot_name, standard_files};
use common::ff1_save::{INVENTORY, KEY_ITEMS, SlotData};
use neoffse_core::{Capability, FileProbe, FormatError, Game, SaveDocument, SaveFiles, SaveFormat, SaveSlot};
use neoffse_data::{Entry, Language};
use neoffse_format_pr::container::{CodecError, Container};
use neoffse_format_pr::document::DocumentError;
use neoffse_format_pr::ff1::{CAPABILITIES, Ff1Document, Ff1Format, OpenError};
use neoffse_format_pr::games::GAMES;
use neoffse_format_pr::slots;

fn source_of<T>(result: Result<T, FormatError>) -> Box<dyn std::error::Error + Send + Sync> {
    match result {
        Err(FormatError::Decode { source }) => source,
        Err(other) => panic!("expected Decode, got {other:?}"),
        Ok(_) => panic!("expected an error"),
    }
}

fn open_error<T>(result: Result<T, FormatError>) -> OpenError {
    *source_of(result).downcast::<OpenError>().expect("the source is an OpenError")
}

fn containers() -> [Container; 3] {
    [
        Container::game(),
        Container { bom: true, trailer: vec![] },
        Container {
            bom: false,
            trailer: b" \r\n\t".to_vec(),
        },
    ]
}

#[test]
fn the_committed_fixture_opens_through_the_codec() {
    let text = String::from_utf8(neoffse_testkit::read_fixture("pr/ff1-slot.json").unwrap()).unwrap();
    let files = one_file(slot_name(1), sav(&text, &Container::game()));
    let mut document = open(&files);
    assert_eq!(document.game(), Game::Ff1Pr);
    assert_eq!(document.capabilities(), CAPABILITIES);
    let summary = document.summary().unwrap();
    assert_eq!(summary.party, ["Aria", "Bram", "Cleo", "Zoé"]);
    assert_eq!(summary.slot, Some(SaveSlot::Numbered(1)));
    let view = document.view().unwrap();
    assert_eq!(view.section(Capability::Party).unwrap().entities.len(), 4);
    assert_eq!(
        view.section(Capability::Inventory).unwrap().entities.len(),
        1 + INVENTORY.len() + KEY_ITEMS.len()
    );
}

#[test]
fn the_trait_open_matches_open_document() {
    let files = standard_files(&Container::game());
    let format: &dyn SaveFormat = &Ff1Format::new();
    let mut boxed = format.open(&files).unwrap();
    let mut typed = open(&files);
    assert_eq!(boxed.summary().unwrap(), typed.summary().unwrap());
    assert_eq!(boxed.view().unwrap(), typed.view().unwrap());
}

#[test]
fn the_container_is_kept() {
    for container in containers() {
        let files = one_file(slot_name(1), sav(&SlotData::standard().render(), &container));
        assert_eq!(open(&files).container(), &container);
    }
}

#[test]
fn any_single_file_name_opens() {
    let files = one_file("copy.sav", sav(&SlotData::standard().render(), &Container::game()));
    assert_eq!(open(&files).file_name(), "copy.sav");
}

#[test]
fn no_file_or_several_files_is_refused() {
    let format = Ff1Format::new();
    assert!(matches!(open_error(format.open_document(&SaveFiles::new())), OpenError::FileCount { found: 0 }));
    let bytes = sav(&SlotData::standard().render(), &Container::game());
    let mut two = one_file(slot_name(1), bytes.clone());
    two.push(slot_name(2), bytes).unwrap();
    assert!(matches!(open_error(format.open_document(&two)), OpenError::FileCount { found: 2 }));
    assert!(matches!(open_error(format.open(&two).map(|_| ())), OpenError::FileCount { found: 2 }));
}

#[test]
fn bytes_that_are_no_container_are_a_decode_error() {
    let format = Ff1Format::new();
    let garbage = one_file(slot_name(1), b"this is not a save \xff\x00".to_vec());
    assert!(source_of(format.open_document(&garbage)).downcast::<CodecError>().is_ok());
    let empty = one_file(slot_name(1), Vec::new());
    let source = source_of(format.open_document(&empty));
    assert!(matches!(source.downcast_ref::<CodecError>(), Some(CodecError::Empty)));
}

#[test]
fn a_container_with_text_that_is_no_json_is_a_decode_error() {
    let files = one_file(slot_name(1), sav("not json at all", &Container::game()));
    let source = source_of(Ff1Format::new().open_document(&files));
    assert!(matches!(source.downcast_ref::<DocumentError>(), Some(DocumentError::InvalidJson { .. })));
}

#[test]
fn a_container_that_is_no_slot_is_refused() {
    let bestiary = r#"{"scenarioFlags":"","monsterDefeats":"","totalSubjugationCount":0}"#.to_owned();
    let index = r#"{"keys":[1],"values":[2]}"#.to_owned();
    let mut no_characters = SlotData::standard();
    no_characters.character_list = Some(embed(r#"{"other":[]}"#));
    let broken_slot = no_characters.render();
    for text in [bestiary, index, broken_slot] {
        let files = one_file(slot_name(1), sav(&text, &Container::game()));
        assert!(matches!(open_error(Ff1Format::new().open_document(&files)), OpenError::NotASlot { .. }));
    }
}

#[test]
fn a_slot_without_the_item_lists_is_not_a_slot() {
    let text = SlotData::standard().render();
    for key in ["normalOwnedItemList", "importantOwendItemList"] {
        let broken = text.replacen(&format!("\\\"{key}\\\""), &format!("\\\"{key}X\\\""), 1);
        assert_ne!(broken, text, "{key}");
        let files = one_file(slot_name(1), sav(&broken, &Container::game()));
        assert!(
            matches!(open_error(Ff1Format::new().open_document(&files)), OpenError::NotASlot { .. }),
            "{key}"
        );
    }
}

#[test]
fn open_keeps_no_borrow() {
    let files = standard_files(&Container::game());
    let mut document = open(&files);
    drop(files);
    assert_eq!(document.summary().unwrap().party.len(), 4);
}

#[test]
fn the_types_cross_threads() {
    fn send_sync<T: Send + Sync>() {}
    fn send<T: Send>() {}
    send_sync::<Ff1Format>();
    send::<Ff1Document>();
    send::<Box<dyn SaveDocument>>();
}

#[test]
fn the_game_id_is_pinned_to_the_games_table() {
    assert_eq!(Ff1Format::new().game().id(), GAMES[0].id);
    assert_eq!(Game::from_id(GAMES[0].id), Some(Game::Ff1Pr));
}

#[test]
fn detect_goes_through_the_slot_table() {
    let format = Ff1Format::new();
    let head = b"AAAA";
    for name in [slot_name(1), ".neoffse-x.tmp"] {
        let probe = FileProbe::new(name, 4, head);
        assert_eq!(format.detect(&probe), slots::detect(&probe), "{name}");
    }
    assert!(format.detect(&FileProbe::new(slot_name(1), 4, head)).is_some());
    assert!(format.detect(&FileProbe::new(".neoffse-x.tmp", 4, head)).is_none());
}

#[test]
fn debug_holds_no_save_content() {
    let document = open(&standard_files(&Container::game()));
    let printed = format!("{document:?}");
    assert!(printed.contains("Ff1Document"));
    assert!(!printed.contains("Aria") && !printed.contains("13621"), "{printed}");
}

#[test]
fn the_language_reaches_the_view() {
    let files = standard_files(&Container::game());
    let tables = neoffse_data::tables_for(Game::Ff1Pr).unwrap();
    let (key, entry): (u32, &Entry) = INVENTORY
        .iter()
        .filter_map(|(id, _)| Some((*id, tables.content(*id)?)))
        .find(|(_, entry)| entry.name(Language::En) != entry.name(Language::Fr))
        .expect("an inventory item with two names");
    let label = |format: Ff1Format| {
        let mut document = format.open_document(&files).unwrap();
        let view = document.view().unwrap();
        let id = neoffse_core::RowId::new(neoffse_core::FieldId::new("inventory"), key);
        let inventory = view.section(Capability::Inventory).unwrap();
        inventory.entities.iter().find(|entity| &entity.id == id.id()).unwrap().label.clone()
    };
    assert_eq!(label(Ff1Format::new()), entry.name(Language::En));
    assert_eq!(label(Ff1Format::with_language(Language::Fr)), entry.name(Language::Fr));
    assert_eq!(Ff1Format::new().language(), Language::En);
    assert_eq!(Ff1Format::with_language(Language::Fr).language(), Language::Fr);
    assert_eq!(Ff1Format::default().language(), Language::En);
}
