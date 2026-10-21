//! Text rules of the committed FF I tables: plain names after the icon tag
//! is stripped (`docs/formats/pixel-remaster.md#game-master-data`) and
//! clean file bytes.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1;
use neoffse_core::{Game, TableKind};
use neoffse_data::sources_for;

const KINDS: [TableKind; 11] = [
    TableKind::Item,
    TableKind::KeyItem,
    TableKind::Weapon,
    TableKind::Shield,
    TableKind::Armor,
    TableKind::Helmet,
    TableKind::Gloves,
    TableKind::Ability,
    TableKind::Job,
    TableKind::Command,
    TableKind::Monster,
];

/// The letters the doc lists for French names, beyond ASCII.
const FRENCH_EXTRA: &str = "ÉâèéêëîïôûŒœ";

fn all_names() -> Vec<(TableKind, u32, &'static str, &'static str)> {
    KINDS
        .iter()
        .flat_map(|kind| {
            ff1()
                .table(*kind)
                .unwrap()
                .iter()
                .map(|entry| (*kind, entry.id, entry.name_en.as_str(), entry.name_fr.as_str()))
        })
        .collect()
}

#[test]
fn names_hold_no_icon_tag_and_no_markup() {
    for (kind, id, en, fr) in all_names() {
        for name in [en, fr] {
            let at = format!("{kind:?} {id}");
            assert!(!name.is_empty(), "{at}");
            assert!(!name.contains(['"', '\\', '\t', '<', '>']), "{at}");
            assert_eq!(name.trim(), name, "{at}");
        }
    }
}

#[test]
fn english_names_are_ascii_and_french_names_add_the_listed_letters() {
    for (kind, id, en, fr) in all_names() {
        assert!(en.is_ascii(), "{kind:?} {id}");
        assert!(fr.chars().all(|c| c.is_ascii() || FRENCH_EXTRA.contains(c)), "{kind:?} {id}");
    }
}

#[test]
fn embedded_text_has_no_bom_and_no_carriage_return() {
    let sources = sources_for(Game::Ff1Pr).unwrap();
    for (file, text) in [
        ("content", sources.content),
        ("abilities", sources.abilities),
        ("jobs", sources.jobs),
        ("commands", sources.commands),
        ("monsters", sources.monsters),
        ("bounds", sources.bounds),
    ] {
        assert!(!text.starts_with('\u{feff}'), "{file}");
        assert!(!text.contains('\r'), "{file}");
        assert!(text.ends_with('\n'), "{file}");
    }
}
