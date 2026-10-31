//! `Ff2Format::open_document` and the trait `open`: the container kept, the
//! game rule, the shape gate and the sources of each error.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::embed;
use common::ff1_format::{one_file, sav, slot_name};
use common::ff1_save;
use common::ff2_save::{SlotData, inventory, key_items, keywords};
use neoffse_core::{Capability, FieldId, FileProbe, FormatError, Game, RowId, SaveDocument, SaveFiles, SaveFormat, SaveSlot};
use neoffse_data::{Entry, Language};
use neoffse_format_pr::OpenError;
use neoffse_format_pr::container::{CodecError, Container};
use neoffse_format_pr::document::DocumentError;
use neoffse_format_pr::ff1::Ff1Format;
use neoffse_format_pr::ff2::{CAPABILITIES, Ff2Document, Ff2Format, Ff2Model, names};
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

fn open(files: &SaveFiles) -> Ff2Document {
    Ff2Format::new().open_document(files).unwrap()
}

fn files_of(text: &str, container: &Container) -> SaveFiles {
    one_file(slot_name(1), sav(text, container))
}

fn standard_files(container: &Container) -> SaveFiles {
    files_of(&SlotData::standard().render(), container)
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
    let text = String::from_utf8(neoffse_testkit::read_fixture("pr/ff2-slot.json").unwrap()).unwrap();
    let mut document = open(&files_of(&text, &Container::game()));
    assert_eq!(document.game(), Game::Ff2Pr);
    assert_eq!(document.capabilities(), CAPABILITIES);
    assert_eq!(CAPABILITIES, [Capability::Party, Capability::Inventory, Capability::Magic]);
    let summary = document.summary().unwrap();
    assert_eq!(summary.party, ["Ivo", "Mara", "Tane"]);
    assert_eq!(summary.slot, Some(SaveSlot::Numbered(1)));
    let view = document.view().unwrap();
    assert_eq!(view.sections.len(), 3);
    // Four Level entities, then the four raw spell pages.
    assert_eq!(view.section(Capability::Magic).unwrap().entities.len(), 8);
    assert_eq!(view.section(Capability::Party).unwrap().entities.len(), 4);
    assert_eq!(
        view.section(Capability::Inventory).unwrap().entities.len(),
        1 + inventory().len() + key_items().len() + keywords().len()
    );
}

#[test]
fn the_trait_open_matches_open_document() {
    let files = standard_files(&Container::game());
    let format: &dyn SaveFormat = &Ff2Format::new();
    let mut boxed = format.open(&files).unwrap();
    let mut typed = open(&files);
    assert_eq!(boxed.summary().unwrap(), typed.summary().unwrap());
    assert_eq!(boxed.view().unwrap(), typed.view().unwrap());
}

#[test]
fn the_container_is_kept() {
    for container in containers() {
        assert_eq!(open(&standard_files(&container)).container(), &container);
    }
}

#[test]
fn any_single_file_name_opens() {
    let files = one_file("copy.sav", sav(&SlotData::standard().render(), &Container::game()));
    assert_eq!(open(&files).file_name(), "copy.sav");
}

#[test]
fn no_file_or_several_files_is_refused() {
    let format = Ff2Format::new();
    assert!(matches!(open_error(format.open_document(&SaveFiles::new())), OpenError::FileCount { found: 0 }));
    let bytes = sav(&SlotData::standard().render(), &Container::game());
    let mut two = one_file(slot_name(1), bytes.clone());
    two.push(slot_name(2), bytes).unwrap();
    assert!(matches!(open_error(format.open_document(&two)), OpenError::FileCount { found: 2 }));
}

#[test]
fn bytes_that_are_no_container_are_a_decode_error() {
    let format = Ff2Format::new();
    let garbage = one_file(slot_name(1), b"this is not a save \xff\x00".to_vec());
    assert!(source_of(format.open_document(&garbage)).downcast::<CodecError>().is_ok());
    let empty = one_file(slot_name(1), Vec::new());
    let source = source_of(format.open_document(&empty));
    assert!(matches!(source.downcast_ref::<CodecError>(), Some(CodecError::Empty)));
}

#[test]
fn a_container_with_text_that_is_no_json_is_a_decode_error() {
    let source = source_of(Ff2Format::new().open_document(&files_of("not json at all", &Container::game())));
    assert!(matches!(source.downcast_ref::<DocumentError>(), Some(DocumentError::InvalidJson { .. })));
}

#[test]
fn a_container_that_is_no_slot_is_refused() {
    let bestiary = r#"{"scenarioFlags":"","monsterDefeats":"","totalSubjugationCount":0}"#.to_owned();
    let index = r#"{"keys":[1],"values":[2]}"#.to_owned();
    let mut no_characters = SlotData::standard();
    no_characters.character_list = Some(embed(r#"{"other":[]}"#));
    for text in [bestiary, index, no_characters.render()] {
        assert!(matches!(
            open_error(Ff2Format::new().open_document(&files_of(&text, &Container::game()))),
            OpenError::NotASlot { .. }
        ));
    }
}

#[test]
fn a_slot_without_the_item_lists_is_not_a_slot() {
    let text = SlotData::standard().render();
    for key in ["normalOwnedItemList", "importantOwendItemList"] {
        let broken = text.replacen(&format!("\\\"{key}\\\""), &format!("\\\"{key}X\\\""), 1);
        assert_ne!(broken, text, "{key}");
        assert!(
            matches!(
                open_error(Ff2Format::new().open_document(&files_of(&broken, &Container::game()))),
                OpenError::NotASlot { .. }
            ),
            "{key}"
        );
    }
}

#[test]
fn a_repeated_character_id_is_not_a_slot() {
    let mut data = SlotData::standard();
    let mut copy = data.characters[0].clone();
    copy.name = "Copy".to_owned();
    data.characters.push(copy);
    let source = source_of(Ff2Format::new().open_document(&files_of(&data.render(), &Container::game())));
    let Some(OpenError::NotASlot { source }) = source.downcast_ref::<OpenError>() else {
        panic!("expected NotASlot")
    };
    let row = RowId::new(FieldId::new(names::CHARACTER), 1);
    assert!(matches!(source.downcast_ref(), Some(neoffse_core::EditError::DuplicateRow { row: found }) if found == row.id()));
}

#[test]
fn a_slot_of_another_game_is_refused_each_way() {
    // An FF II slot reaches the FF I format: the vehicle count names FF II.
    let ff2 = standard_files(&Container::game());
    assert!(matches!(
        open_error(Ff1Format::new().open_document(&ff2)),
        OpenError::OtherGame {
            found: Some(Game::Ff2Pr),
            vehicles: 7
        }
    ));
    // An FF I slot reaches the FF II format.
    let ff1 = one_file(slot_name(1), sav(&ff1_save::SlotData::standard().render(), &Container::game()));
    assert!(matches!(
        open_error(Ff2Format::new().open_document(&ff1)),
        OpenError::OtherGame { found: Some(Game::Ff1Pr), .. }
    ));
    assert!(Ff2Format::new().open_document(&ff2).is_ok());
}

#[test]
fn a_vehicle_count_of_no_game_is_refused() {
    for vehicles in [5, 14] {
        let data = SlotData {
            vehicles,
            ..SlotData::standard()
        };
        let result = Ff2Format::new().open_document(&files_of(&data.render(), &Container::game()));
        assert!(
            matches!(open_error(result), OpenError::OtherGame { vehicles: found, .. } if found == vehicles),
            "{vehicles} vehicles"
        );
    }
}

#[test]
fn open_keeps_no_borrow() {
    let files = standard_files(&Container::game());
    let mut document = open(&files);
    drop(files);
    assert_eq!(document.summary().unwrap().party.len(), 3);
}

#[test]
fn the_types_cross_threads() {
    fn send_sync<T: Send + Sync>() {}
    fn send<T: Send>() {}
    send_sync::<Ff2Format>();
    send::<Ff2Document>();
    send::<Box<dyn SaveDocument>>();
}

#[test]
fn the_game_id_is_pinned_to_the_games_table() {
    assert_eq!(Ff2Format::new().game().id(), GAMES[1].id);
    assert_eq!(Game::from_id(GAMES[1].id), Some(Game::Ff2Pr));
    // The generic format builds without a panic: the embedded data holds the FF II tables.
    assert_eq!(neoffse_format_pr::shared::SlotFormat::<Ff2Model>::new().game(), Game::Ff2Pr);
}

#[test]
fn detect_goes_through_the_slot_table() {
    let format = Ff2Format::new();
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
    let mut document = open(&standard_files(&Container::game()));
    // The opened document holds the names and the play time the Debug text must hide.
    assert_eq!(document.summary().unwrap().party, ["Ivo", "Mara", "Tane"]);
    assert!(document.summary().unwrap().play_time.is_some_and(|time| time.as_secs() == 13_621));
    let printed = format!("{document:?}");
    assert!(printed.contains("Ff2Document"));
    for hidden in ["Ivo", "Mara", "Tane", "Élan", "13621", "1234567"] {
        assert!(!printed.contains(hidden), "Debug prints the value {hidden}");
    }
}

#[test]
fn the_language_reaches_the_view() {
    let files = standard_files(&Container::game());
    let tables = neoffse_data::tables_for(Game::Ff2Pr).unwrap();
    let (key, entry): (u32, &Entry) = inventory()
        .iter()
        .filter_map(|(id, _)| Some((*id, tables.content(*id)?)))
        .find(|(_, entry)| entry.name(Language::En).is_some() && entry.name(Language::En) != entry.name(Language::Fr))
        .expect("an inventory item with two names");
    let label = |format: Ff2Format| {
        let mut document = format.open_document(&files).unwrap();
        let view = document.view().unwrap();
        let id = RowId::new(FieldId::new(names::INVENTORY), key);
        let inventory = view.section(Capability::Inventory).unwrap();
        inventory.entities.iter().find(|entity| &entity.id == id.id()).unwrap().label.clone()
    };
    assert_eq!(Some(label(Ff2Format::new()).as_str()), entry.name(Language::En));
    assert_eq!(Some(label(Ff2Format::with_language(Language::Fr)).as_str()), entry.name(Language::Fr));
    assert_eq!(Ff2Format::new().language(), Language::En);
    assert_eq!(Ff2Format::with_language(Language::Fr).language(), Language::Fr);
    assert_eq!(Ff2Format::default().language(), Language::En);
}
