//! The ability row contract: a required `type_id`, an optional `content_id`
//! and the sixteen kind strings (`docs/formats/pixel-remaster.md#ability-types`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{minimal, rule};
use neoffse_core::{Game, TableKind};
use neoffse_data::{AbilityKind, DataError, GameTables, Sources};

/// The sixteen kind strings with their variant and one `type_id` each.
const KINDS: [(&str, AbilityKind, u32); 16] = [
    ("white_magic", AbilityKind::WhiteMagic, 1),
    ("black_magic", AbilityKind::BlackMagic, 2),
    ("monster", AbilityKind::Monster, 3),
    ("command", AbilityKind::Command, 4),
    ("summon", AbilityKind::Summon, 9),
    ("ninjutsu", AbilityKind::Ninjutsu, 10),
    ("other", AbilityKind::Other, 11),
    ("time_magic", AbilityKind::TimeMagic, 501),
    ("magic_sword", AbilityKind::MagicSword, 502),
    ("blue_magic", AbilityKind::BlueMagic, 504),
    ("hissatsu", AbilityKind::Hissatsu, 601),
    ("special_ability", AbilityKind::SpecialAbility, 602),
    ("dance", AbilityKind::Dance, 603),
    ("madou", AbilityKind::Madou, 604),
    ("rampage", AbilityKind::Rampage, 625),
    ("internal", AbilityKind::Internal, 12),
];

fn parse(abilities: &str) -> Result<GameTables, DataError> {
    GameTables::parse(Game::Ff1Pr, &Sources { abilities, ..minimal() })
}

#[test]
fn every_kind_string_parses() {
    let rows: Vec<String> = KINDS
        .iter()
        .zip(1_u32..)
        .map(|((kind, _, type_id), id)| {
            let content_id = 100 + id;
            format!("[[ability]]\nid = {id}\ncontent_id = {content_id}\ntype_id = {type_id}\nkind = \"{kind}\"\nlevel = 0\n\n")
        })
        .collect();
    let text = rows.concat();
    let tables = parse(&text).unwrap();
    let rows = tables.table(TableKind::Ability).unwrap();
    assert_eq!(rows.len(), 16);
    for (row, (_, kind, type_id)) in rows.iter().zip(KINDS) {
        let info = row.ability.unwrap();
        assert_eq!((info.kind, info.type_id), (kind, type_id), "row {}", row.id);
        assert_eq!(info.content_id, Some(100 + row.id));
    }
}

#[test]
fn an_ability_without_a_type_id_is_a_toml_error() {
    let text = "[[ability]]\nid = 1\ncontent_id = 209\nkind = \"command\"\nlevel = 0\n";
    match parse(text).unwrap_err() {
        DataError::Toml { file, .. } => assert_eq!(file, "abilities.toml"),
        other => panic!("expected a Toml error, got {other}"),
    }
}

#[test]
fn an_ability_without_a_content_id_is_valid_and_unfindable() {
    let text = "[[ability]]\nid = 1\ncontent_id = 209\ntype_id = 4\nkind = \"command\"\nlevel = 0\n\n\
                [[ability]]\nid = 2\ntype_id = 625\nkind = \"rampage\"\nlevel = 0\n";
    let tables = parse(text).unwrap();
    let nameless = tables.entry(TableKind::Ability, 2).unwrap().ability.unwrap();
    assert_eq!((nameless.content_id, nameless.type_id, nameless.kind), (None, 625, AbilityKind::Rampage));
    assert_eq!(tables.ability_by_content(209).unwrap().id, 1);
    assert!(tables.ability_by_content(2).is_none());
    assert!(tables.ability_by_content(0).is_none());
}

#[test]
fn a_content_id_of_zero_is_still_refused() {
    let text = "[[ability]]\nid = 1\ncontent_id = 0\ntype_id = 4\nkind = \"command\"\nlevel = 0\n";
    assert_eq!(rule(parse(text)), ("abilities.toml", "id 1".to_string(), "content_id 0"));
}

#[test]
fn an_unknown_kind_is_still_refused() {
    let text = "[[ability]]\nid = 1\ncontent_id = 209\ntype_id = 4\nkind = \"spell\"\nlevel = 0\n";
    match parse(text).unwrap_err() {
        DataError::UnknownKind { file, id, kind } => assert_eq!((file, id, kind.as_str()), ("abilities.toml", 1, "spell")),
        other => panic!("expected UnknownKind, got {other}"),
    }
}
