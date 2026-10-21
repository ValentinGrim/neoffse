//! The FF I ability rules of `docs/formats/pixel-remaster.md#abilities`,
//! checked on a slot's JSON tree. A message carries a path and never a value,
//! so the checks can also run on private saves.

use neoffse_format_pr::ff1::GAME;

use super::json_tree::Tree;

const CHARS: &str = "userData.ownedCharacterList.target";
/// `contentId - abilityId` for FF I spells.
const OFFSET: i64 = 208;
const SLOTS_PER_LEVEL: usize = 3;

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

/// Every breach of the rules for the characters of `tree`, one message each.
pub(crate) fn ability_violations(tree: &Tree) -> Vec<String> {
    let mut found = Vec::new();
    let mut learned = Vec::new();
    for character in 0..tree.array_len(CHARS).unwrap_or(0) {
        let base = format!("{CHARS}[{character}]");
        let list = format!("{base}.abilityList.target");
        let (Some(abilities), Some(contents)) = (ints(tree, &list, "abilityId"), ints(tree, &list, "contentId")) else {
            found.push(format!("{list}: unreadable"));
            continue;
        };
        for index in 0..abilities.len() {
            if tree.int(&format!("{list}[{index}].skillLevel")) != Some(0) {
                found.push(format!("{list}[{index}].skillLevel: not 0"));
            }
            if contents[index] != abilities[index] + OFFSET {
                found.push(format!("{list}[{index}].contentId: not abilityId + {OFFSET}"));
            }
        }
        let order = format!("{base}.additionOrderOwnedAbilityIds.target");
        if ints(tree, &order, "") != Some(contents.clone()) {
            found.push(format!("{order}: not the contentIds of abilityList"));
        }
        found.extend(slot_violations(tree, &base, &abilities, &contents));
        learned.extend(abilities);
    }
    learned.sort_unstable();
    learned.dedup();
    // The doc states the union and no order; real saves list the IDs unsorted.
    let mut stored = ints(tree, "userData.learnedAbilityList.target", "").unwrap_or_default();
    stored.sort_unstable();
    if stored != learned {
        found.push("userData.learnedAbilityList.target: not the union of the party's abilityIds".to_owned());
    }
    found
}

/// The spells of one `abilitySlotDataList` against its `abilityList`.
fn slot_violations(tree: &Tree, base: &str, abilities: &[i64], contents: &[i64]) -> Vec<String> {
    let mut found = Vec::new();
    let tables = neoffse_data::tables_for(GAME);
    let mut slotted = Vec::new();
    for level in 0..tree.array_len(&format!("{base}.abilitySlotDataList.target")).unwrap_or(0) {
        let info = format!("{base}.abilitySlotDataList.target[{level}].slotInfo");
        let keys = ints(tree, &format!("{info}.keys"), "");
        let values = tree.array_len(&format!("{info}.values"));
        if keys != Some((1..).take(SLOTS_PER_LEVEL).collect()) || values != Some(SLOTS_PER_LEVEL) {
            found.push(format!("{info}: not keys 1 to 3 with three values"));
            continue;
        }
        for slot in 0..SLOTS_PER_LEVEL {
            let value = format!("{info}.values[{slot}]");
            if tree.raw_at(&value) == Some("\"\"") {
                continue;
            }
            let Some(ability) = tree.int(&format!("{value}.abilityId")) else {
                found.push(format!("{value}: neither \"\" nor an ability"));
                continue;
            };
            let at = contents.get(abilities.iter().position(|id| *id == ability).unwrap_or(usize::MAX));
            let spell_level = at
                .and_then(|content| u32::try_from(*content).ok())
                .and_then(|content| tables?.ability_by_content(content))
                .and_then(|entry| entry.ability)
                .map(|info| usize::from(info.level));
            if at.is_none() {
                found.push(format!("{value}: not in abilityList"));
            } else if spell_level != Some(level + 1) {
                found.push(format!("{value}: sits at the wrong spell level"));
            }
            slotted.push(ability);
        }
    }
    for (index, ability) in abilities.iter().enumerate() {
        if slotted.iter().filter(|id| *id == ability).count() != 1 {
            found.push(format!("{base}.abilityList.target[{index}]: not in exactly one slot"));
        }
    }
    found
}
