//! The FF II ability rules of `docs/formats/pixel-remaster.md#abilities` and
//! `#row-and-value-in-ff-ii`, checked on a slot's JSON tree. A message
//! carries a path and never a value, so the checks can also run on private
//! saves.
//!
//! The rules leave `skillLevel` out: the row and the value are two facts, and
//! an editor pass may leave them apart (`#row-and-value-in-ff-ii`).

use neoffse_core::TableKind;
use neoffse_format_pr::ff2::GAME;

use super::exit_facts::CHARS;
use super::json_tree::Tree;

/// The internal entry every character holds, outside the Magic list.
const INTERNAL: i64 = 711;
/// The key of the Magic command in `abilityDictionary`.
const MAGIC_COMMAND: i64 = 3;

fn ints(tree: &Tree, array: &str, key: &str) -> Option<Vec<i64>> {
    (0..tree.array_len(array)?)
        .map(|index| {
            tree.int(&if key.is_empty() {
                format!("{array}[{index}]")
            } else {
                format!("{array}[{index}].{key}")
            })
        })
        .collect()
}

/// The content id of the type-4 row of `ability` in the tables.
fn content_of(ability: i64) -> Option<i64> {
    let tables = neoffse_data::tables_for(GAME)?;
    let entry = tables.entry(TableKind::Ability, u32::try_from(ability).ok()?)?;
    Some(i64::from(entry.ability?.content_id?))
}

/// Every breach of the rules for the characters of `tree`, one message each:
/// `contentId` is the content row of its `abilityId` (join through the
/// tables, never an offset) and an `abilityId` occurs once; the addition order
/// lists the content ids of `abilityList` in order; the sort order holds
/// owned content ids only; the key-3 dictionary list holds the ids of
/// `abilityList` minus 711, in order.
pub(crate) fn ability_violations(tree: &Tree) -> Vec<String> {
    let mut found = Vec::new();
    for character in 0..tree.array_len(CHARS).unwrap_or(0) {
        let base = format!("{CHARS}[{character}]");
        let list = format!("{base}.abilityList.target");
        let (Some(abilities), Some(contents)) = (ints(tree, &list, "abilityId"), ints(tree, &list, "contentId")) else {
            found.push(format!("{list}: unreadable"));
            continue;
        };
        for (index, ability) in abilities.iter().enumerate() {
            if content_of(*ability) != Some(contents[index]) {
                found.push(format!("{list}[{index}].contentId: not the content row of the abilityId"));
            }
            if abilities[..index].contains(ability) {
                found.push(format!("{list}[{index}].abilityId: repeats"));
            }
        }
        let order = format!("{base}.additionOrderOwnedAbilityIds.target");
        if ints(tree, &order, "") != Some(contents.clone()) {
            found.push(format!("{order}: not the contentIds of abilityList"));
        }
        let sorted = format!("{base}.sortOrderOwnedAbilityIds.target");
        match ints(tree, &sorted, "") {
            Some(ids) if ids.iter().all(|id| contents.contains(id)) => {}
            _ => found.push(format!("{sorted}: not a subset of the owned contentIds")),
        }
        found.extend(dictionary_violations(tree, &base, &abilities));
    }
    found
}

/// The key-3 list against `abilityList`: absent for a character with no spell,
/// else the same ids minus 711 in the same order.
fn dictionary_violations(tree: &Tree, base: &str, abilities: &[i64]) -> Option<String> {
    let spells: Vec<i64> = abilities.iter().copied().filter(|id| *id != INTERNAL).collect();
    let dictionary = format!("{base}.abilityDictionary");
    let copy = tree
        .find_int(&format!("{dictionary}.keys"), MAGIC_COMMAND)
        .map(|index| format!("{dictionary}.values[{index}].target"));
    match copy {
        None if spells.is_empty() => None,
        None => Some(format!("{dictionary}: no key {MAGIC_COMMAND} for a character with spells")),
        Some(array) if ints(tree, &array, "abilityId") == Some(spells) => None,
        Some(array) => Some(format!("{array}: not the abilityList minus 711")),
    }
}
