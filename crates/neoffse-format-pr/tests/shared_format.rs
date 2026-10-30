//! `SlotFormat` through `Ff1Model`: the alias names the generic type, the
//! hand-written traits hold without a bound on the model, and the language
//! reaches the view labels.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_format::{slot_name, standard_files};
use common::ff1_save::INVENTORY;
use neoffse_core::{Capability, FieldId, FileProbe, Game, RowId, SaveDocument, SaveFormat};
use neoffse_data::{Entry, Language, tables_for};
use neoffse_format_pr::container::Container;
use neoffse_format_pr::ff1::{Ff1Format, Ff1Model};
use neoffse_format_pr::shared::{SlotFormat, SlotModel};
use neoffse_format_pr::slots;

fn assert_send_sync<T: Send + Sync>() {}

fn assert_copy<T: Copy>() {}

fn cloned<T: Clone>(value: &T) -> T {
    value.clone()
}

/// The game the format of `M` reports, read through the trait alone.
fn game_of<M: SlotModel>() -> Game {
    SaveFormat::game(&SlotFormat::<M>::new())
}

#[test]
fn the_alias_is_the_generic_format() {
    let _: SlotFormat<Ff1Model> = Ff1Format::new();
    assert_eq!(Ff1Format::new().game(), Game::Ff1Pr);
    assert_eq!(game_of::<Ff1Model>(), Game::Ff1Pr);
}

#[test]
fn the_format_is_copy_clone_default_and_thread_safe() {
    assert_copy::<SlotFormat<Ff1Model>>();
    assert_send_sync::<SlotFormat<Ff1Model>>();
    let format = Ff1Format::with_language(Language::Fr);
    let copy = format;
    let clone = cloned(&format);
    assert_eq!(copy.language(), Language::Fr);
    assert_eq!(clone.language(), Language::Fr);
    assert_eq!(Ff1Format::default().language(), Language::En);
}

#[test]
fn debug_names_the_game_and_the_language_only() {
    let printed = format!("{:?}", Ff1Format::new());
    assert!(printed.contains("SlotFormat"));
    assert!(printed.contains("Ff1Pr"));
    assert!(printed.contains("En"));
    // The tables hold item names: the Debug text must not print them.
    let tables = tables_for(Game::Ff1Pr).unwrap();
    let name = INVENTORY.iter().find_map(|(id, _)| tables.content(*id)?.name(Language::En)).unwrap();
    assert!(!printed.contains(name), "Debug prints the table name {name}");
}

#[test]
fn the_generic_format_opens_and_detects_like_the_alias() {
    let files = standard_files(&Container::game());
    let format = SlotFormat::<Ff1Model>::new();
    let document = format.open_document(&files).unwrap();
    assert_eq!(document.file_name(), slot_name(1));
    assert_eq!(document.game(), Game::Ff1Pr);
    assert_eq!(document.encode().unwrap().iter().next().unwrap().1, files.iter().next().unwrap().1);

    let boxed = format.open(&files).unwrap();
    assert_eq!(boxed.game(), Game::Ff1Pr);
    assert!(boxed.capabilities().contains(&Capability::Inventory));

    let probe = FileProbe::new(slot_name(1), 4, b"AAAA");
    assert_eq!(format.detect(&probe), slots::detect(&probe));
    assert!(format.detect(&probe).is_some());
    assert!(format.detect(&FileProbe::new(".neoffse-x.tmp", 4, b"AAAA")).is_none());
}

#[test]
fn the_language_reaches_the_view_through_the_generic_name() {
    let files = standard_files(&Container::game());
    let tables = tables_for(Game::Ff1Pr).unwrap();
    let (key, entry): (u32, &Entry) = INVENTORY
        .iter()
        .filter_map(|(id, _)| Some((*id, tables.content(*id)?)))
        .find(|(_, entry)| entry.name(Language::En) != entry.name(Language::Fr))
        .expect("an inventory item with two names");
    let label = |format: SlotFormat<Ff1Model>| {
        let mut document = format.open_document(&files).unwrap();
        let view = document.view().unwrap();
        let id = RowId::new(FieldId::new("inventory"), key);
        let inventory = view.section(Capability::Inventory).unwrap();
        inventory.entities.iter().find(|entity| &entity.id == id.id()).unwrap().label.clone()
    };
    let english = SlotFormat::<Ff1Model>::with_language(Language::En);
    let french = SlotFormat::<Ff1Model>::with_language(Language::Fr);
    assert_eq!(Some(label(english).as_str()), entry.name(Language::En));
    assert_eq!(Some(label(french).as_str()), entry.name(Language::Fr));
    assert_ne!(label(english), label(french));
}
