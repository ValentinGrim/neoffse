//! The nameless rows of the committed FF VI tables (build 20265119), after
//! `docs/formats/pixel-remaster.md#rows-without-a-name` and `#master-data-per-game`, "Repeated IDs".
//! The counts come from the document, not from the file.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;

use common::game_tables;
use neoffse_core::{Game, TableKind};
use neoffse_data::{AbilityKind, Entry, GameTables, Language};

fn ff6() -> &'static GameTables {
    game_tables(Game::Ff6Pr)
}

fn table(kind: TableKind) -> &'static [Entry] {
    ff6().table(kind).unwrap()
}

fn nameless(kind: TableKind) -> Vec<u32> {
    table(kind).iter().filter(|entry| entry.name_en.is_none()).map(|entry| entry.id).collect()
}

/// The 89 ability rows whose name key sits in a story file (`#rows-without-a-name`), as pairs of
/// first and last ID.
fn story_ability_ids() -> Vec<u32> {
    let pairs: [u32; 40] = [
        1096, 1096, 1111, 1131, 1147, 1155, 1163, 1165, 1169, 1171, 1181, 1182, 1188, 1189, 1258, 1258, 1267, 1279, 1306, 1306, 1316, 1319, 1321, 1321, 1323,
        1323, 1326, 1326, 1334, 1335, 1345, 1349, 1353, 1354, 1360, 1368, 1370, 1374, 1421, 1423,
    ];
    pairs.chunks(2).flat_map(|pair| pair[0]..=pair[1]).collect()
}

/// The 149 abilities with no content row in the game: the 145 of the file plus 1525, 1527, 1528
/// and 1529, which the repeated content IDs 1861 and 1863 overwrite.
fn contentless_ids() -> Vec<u32> {
    [1525..=1525, 1527..=1529, 5000..=5100, 5102..=5127, 5900..=5917]
        .into_iter()
        .flatten()
        .collect()
}

#[test]
fn the_three_causes_of_a_nameless_ability() {
    // `#rows-without-a-name`: 313 rows with key None, 90 with a key in no `system_*` file (431
    // and the 89 story-key rows), 149 with no content row. The three sets are disjoint.
    for entry in table(TableKind::Ability) {
        assert_eq!(entry.name_en.is_none(), entry.name_fr.is_none(), "ability {}", entry.id);
    }
    let none = nameless(TableKind::Ability);
    assert_eq!(none.len(), 313 + 90 + 149);
    let contentless = contentless_ids();
    assert_eq!(contentless.len(), 149);
    let mut story = story_ability_ids();
    assert_eq!(story.len(), 89);
    story.push(431);
    for id in contentless.iter().chain(&story) {
        let entry = ff6().entry(TableKind::Ability, *id).unwrap();
        assert!(entry.name_en.is_none(), "ability {id}");
    }
    let with_content = |id: &u32| ff6().entry(TableKind::Ability, *id).unwrap().ability.unwrap().content_id.is_some();
    assert!(story.iter().all(with_content));
    assert!(!contentless.iter().any(with_content));
    // The rest of the nameless rows are the 313 with key None.
    let rest = none.iter().filter(|id| !contentless.contains(id) && !story.contains(id));
    assert_eq!(rest.count(), 313);
    let named = table(TableKind::Ability).iter().filter(|entry| entry.name_en.is_some()).count();
    assert_eq!(named, 1631 - 552);
}

#[test]
fn story_key_and_contentless_rows_have_the_documented_types() {
    for id in story_ability_ids() {
        let info = ff6().entry(TableKind::Ability, id).unwrap().ability.unwrap();
        assert_eq!((info.type_id, info.kind), (34, AbilityKind::Internal), "ability {id}");
    }
    for id in [5000..=5100, 5102..=5127, 5900..=5916].into_iter().flatten() {
        let info = ff6().entry(TableKind::Ability, id).unwrap().ability.unwrap();
        assert_eq!((info.type_id, info.kind, info.content_id), (35, AbilityKind::Internal, None), "ability {id}");
    }
    let info = ff6().entry(TableKind::Ability, 5917).unwrap().ability.unwrap();
    assert_eq!((info.type_id, info.kind, info.content_id), (41, AbilityKind::Internal, None));
    assert!(ff6().entry(TableKind::Ability, 5101).is_none());
}

#[test]
fn the_last_row_of_a_repeated_content_id_wins() {
    // `#master-data-per-game`, "Repeated IDs": content 1861 is ability 1530 and content 1863 is
    // ability 1531; abilities 1525, 1527, 1528 and 1529 lose their content row and their name.
    let content_of = |id| ff6().entry(TableKind::Ability, id).unwrap().ability.unwrap().content_id;
    assert_eq!((content_of(1530), content_of(1531)), (Some(1861), Some(1863)));
    assert_eq!(ff6().ability_by_content(1861).map(|entry| entry.id), Some(1530));
    assert_eq!(ff6().ability_by_content(1863).map(|entry| entry.id), Some(1531));
    for id in [1525, 1527, 1528, 1529] {
        let entry = ff6().entry(TableKind::Ability, id).unwrap();
        assert!(entry.name_en.is_none() && content_of(id).is_none(), "ability {id}");
    }
    // Ability 431 has a content ID and no name.
    let entry = ff6().entry(TableKind::Ability, 431).unwrap();
    assert!(entry.name_en.is_none() && entry.ability.unwrap().content_id.is_some());
}

#[test]
fn jobs_20_and_22_are_named_from_the_story_file() {
    // Kinds note, section 4: one word of 6 and 7 characters in English; every job is named.
    assert_eq!(nameless(TableKind::Job), Vec::<u32>::new());
    for (id, length) in [(20, 6), (22, 7)] {
        let entry = ff6().entry(TableKind::Job, id).unwrap();
        let en = entry.name(Language::En).unwrap();
        assert_eq!(en.chars().count(), length, "job {id}");
        assert!(!en.contains(' ') && !en.ends_with(|c: char| !c.is_alphanumeric()), "job {id}");
        assert!(entry.name(Language::Fr).is_some_and(|fr| !fr.is_empty()), "job {id}");
    }
    for entry in table(TableKind::Job).iter().chain(table(TableKind::Command)) {
        assert_eq!(entry.name_en.is_none(), entry.name_fr.is_none(), "{:?} {}", entry.kind, entry.id);
    }
}

#[test]
fn command_4_has_no_name() {
    // Saves hold command 4 in `commandList`; the 695 others are named.
    assert_eq!(nameless(TableKind::Command), [4]);
}

#[test]
fn every_content_id_is_unique() {
    let mut seen = BTreeMap::new();
    for entry in table(TableKind::Ability) {
        let Some(content_id) = entry.ability.unwrap().content_id else { continue };
        assert_ne!(content_id, 0);
        assert_eq!(seen.insert(content_id, entry.id), None, "content {content_id}");
    }
    assert_eq!(seen.len(), 1631 - 149);
    for (content_id, id) in &seen {
        assert_eq!(ff6().ability_by_content(*content_id).map(|entry| entry.id), Some(*id));
    }
    assert_eq!(ff6().ability_by_content(0), None);
}
