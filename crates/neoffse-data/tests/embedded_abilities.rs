//! Abilities and commands of the committed FF I tables (build 20249397),
//! after `docs/formats/pixel-remaster.md#abilities` and
//! `#ability-job-and-command-tables`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;

use common::{ff1, kind_rule::kind_of};
use neoffse_core::{Game, TableKind};
use neoffse_data::{AbilityKind, Language, tables_for};

fn ability(id: u32) -> &'static neoffse_data::Entry {
    ff1().entry(TableKind::Ability, id).unwrap()
}

fn name_en(kind: TableKind, id: u32) -> &'static str {
    ff1().entry(kind, id).unwrap().name(Language::En).unwrap()
}

#[test]
fn ability_100_sits_at_content_310() {
    // Ability 100 breaks the rule content id = id + 208.
    let curaja = ability(100);
    let info = curaja.ability.unwrap();
    assert_eq!((info.content_id, info.level, info.kind), (Some(310), 7, AbilityKind::WhiteMagic));
    assert_eq!((curaja.name_en.as_deref(), curaja.name_fr.as_deref()), (Some("Curaja"), Some("Soin Max")));
    assert_eq!(ff1().ability_by_content(310).unwrap().id, 100);
    // Ability content ids are no content-table ids.
    assert!(ff1().content(310).is_none());
    assert!(ff1().entry(TableKind::Ability, 310).is_none());
}

#[test]
fn the_offset_rule_has_one_exception() {
    let abilities = ff1().table(TableKind::Ability).unwrap();
    let off_rule: Vec<u32> = abilities
        .iter()
        .filter(|entry| entry.ability.unwrap().content_id != Some(entry.id + 208))
        .map(|entry| entry.id)
        .collect();
    assert_eq!(off_rule, [100]);
    assert_eq!(abilities.len() - off_rule.len(), 95);
    let ids: Vec<u32> = abilities.iter().map(|entry| entry.id).collect();
    let expected: Vec<u32> = (1..=95).chain([100]).collect();
    assert_eq!(ids, expected);
}

#[test]
fn spell_level_and_school_follow_the_id() {
    for id in 5..=68 {
        let info = ability(id).ability.unwrap();
        assert_eq!(u32::from(info.level), (id - 5) / 8 + 1, "level of {id}");
        let school = if (id - 5) % 8 < 4 { AbilityKind::WhiteMagic } else { AbilityKind::BlackMagic };
        assert_eq!(info.kind, school, "school of {id}");
    }
    for entry in ff1().table(TableKind::Ability).unwrap() {
        let found = ff1().ability_by_content(entry.ability.unwrap().content_id.unwrap()).unwrap();
        assert_eq!(found, entry);
    }
}

#[test]
fn repeated_names_keep_every_id() {
    let (first, second) = (ability(53), ability(100));
    assert_eq!((first.name_en.as_deref(), second.name_en.as_deref()), (Some("Curaja"), Some("Curaja")));
    assert_ne!(first.id, second.id);
    assert_ne!(first.ability.unwrap().content_id, second.ability.unwrap().content_id);
    assert_eq!(second.name_fr.as_deref(), Some("Soin Max"));
    assert_eq!(name_en(TableKind::Monster, 120), "Lich");
    assert_eq!(name_en(TableKind::Monster, 121), "Lich");
    for (id, name) in [
        (122, "Marilith"),
        (123, "Marilith"),
        (124, "Kraken"),
        (125, "Kraken"),
        (126, "Tiamat"),
        (127, "Tiamat"),
    ] {
        assert_eq!(name_en(TableKind::Monster, id), name, "monster {id}");
    }
}

#[test]
fn ability_names_the_go_editor_got_wrong() {
    for (id, name) in [(13, "Invis"), (15, "Blindna"), (66, "Warp"), (67, "Stop")] {
        assert_eq!(name_en(TableKind::Ability, id), name, "ability {id}");
    }
    let commands = ["Attack", "Defend", "Magic", "Items", "Flee", "Back"];
    for (index, name) in commands.iter().enumerate() {
        let id = u32::try_from(index + 1).unwrap();
        assert_eq!(name_en(TableKind::Command, id), *name, "command {id}");
    }
}

#[test]
fn monster_and_command_abilities_have_level_0() {
    let abilities = ff1().table(TableKind::Ability).unwrap();
    let flat: Vec<_> = abilities
        .iter()
        .filter(|entry| matches!(entry.ability.unwrap().kind, AbilityKind::Monster | AbilityKind::Command))
        .collect();
    assert_eq!(flat.len(), 31);
    for entry in flat {
        assert_eq!(entry.ability.unwrap().level, 0, "ability {}", entry.id);
    }
}

#[test]
fn the_ff1_type_ids_match_the_document() {
    // docs/formats/pixel-remaster.md#master-data-per-game, "Abilities".
    let mut histogram = BTreeMap::new();
    for entry in ff1().table(TableKind::Ability).unwrap() {
        *histogram.entry(entry.ability.unwrap().type_id).or_insert(0_usize) += 1;
    }
    let expected = BTreeMap::from([(1, 33), (2, 32), (3, 2), (4, 1), (5, 1), (6, 2), (8, 1), (13, 24)]);
    assert_eq!(histogram, expected);
    assert_eq!(histogram.values().sum::<usize>(), 96);
}

#[test]
fn every_embedded_ability_follows_the_kind_rule() {
    for game in Game::ALL {
        let Some(tables) = tables_for(*game) else { continue };
        for entry in tables.table(TableKind::Ability).unwrap_or_default() {
            let info = entry.ability.unwrap();
            assert_eq!(kind_of(info.type_id), Some(info.kind), "{game:?} ability {}", entry.id);
        }
    }
}
