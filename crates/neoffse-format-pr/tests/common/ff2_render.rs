//! The JSON text of a synthetic FF II slot: the nesting of the string
//! documents and the key order of `docs/formats/pixel-remaster.md#slot-keys`
//! and `docs/formats/pixel-remaster.md#characters`. FF II shares FF I's key
//! set (`docs/formats/pixel-remaster.md#telling-the-games-apart`). The
//! ability keys follow `docs/formats/pixel-remaster.md#abilities` and
//! `docs/formats/pixel-remaster.md#spells-in-ff-ii`.

use super::ff1_render::{CHEAT, config_data, data_storage, dict, list, map_data, numbers, object};
use super::ff2_save::{Character, Row, SlotData, Spell};
use super::{VEHICLE, embed};

/// Ability 711, first in every `abilityList`: the two-sword permission, with
/// content row 937 (`docs/formats/pixel-remaster.md#nameless-rows-in-ff-ii`).
const TWO_SWORD: (u32, u32) = (711, 937);

/// The whole slot, top level included.
pub(super) fn slot(slot: &SlotData) -> String {
    let colon = if slot.spaced { ": " } else { ":" };
    let mut pairs: Vec<(&str, String)> = vec![("id", "1".to_owned()), ("pictureData", "\"UE5HLWludmVudGVk\"".to_owned())];
    if slot.with_user_data {
        pairs.push(("userData", embed(&user_data(slot))));
    }
    pairs.push(("configData", embed(&config_data())));
    pairs.push(("dataStorage", embed(&data_storage())));
    pairs.push(("mapData", embed(&map_data())));
    pairs.push(("timeStamp", "\"3/17/2026 6:42:09 PM\"".to_owned()));
    pairs.push(("playTime", "0.0".to_owned()));
    pairs.push(("clearFlag", "0".to_owned()));
    pairs.push(("isCompleteFlag", "0".to_owned()));
    object(&pairs, colon)
}

fn user_data(slot: &SlotData) -> String {
    let characters = slot.character_list.clone().unwrap_or_else(|| {
        list(
            &slot
                .characters
                .iter()
                .map(|character| embed(&render_character(character, slot)))
                .collect::<Vec<_>>(),
        )
    });
    let corps: Vec<String> = slot.corps.iter().map(|id| embed(&format!("{{\"id\":1,\"characterId\":{id}}}"))).collect();
    let corps_list = list(&corps);
    let rows = |rows: &[Row]| embed(&list(&rows.iter().map(|row| embed(&row.text)).collect::<Vec<_>>()));
    let empty = || embed(&list(&[]));
    let numbers_list = |ids: &[u32]| embed(&list(&ids.iter().map(ToString::to_string).collect::<Vec<_>>()));
    let mut pairs: Vec<(&str, String)> = vec![
        ("corpsList", embed(&corps_list)),
        ("corpsSlots", embed(&dict(&["0", "1", "2"].map(String::from), &vec![embed(&corps_list); 3]))),
        ("ownedCharacterList", embed(&characters)),
        ("releasedJobs", numbers_list(&released_jobs(slot))),
    ];
    if let Some(gil) = &slot.gil {
        pairs.push(("owendGil", gil.clone()));
    }
    pairs.extend([
        ("playTime", slot.play_time.clone()),
        ("normalOwnedItemList", rows(&slot.inventory)),
        ("importantOwendItemList", rows(&slot.key_items)),
        ("normalOwnedItemSortIdList", numbers_list(&slot.sort_ids)),
        ("currentArea", "\"Invented Plains\"".to_owned()),
        ("currentLocation", "\"\"".to_owned()),
        ("ownedTransportationList", embed(&list(&vec![embed(VEHICLE); slot.vehicles]))),
        ("owendCrystalFlags", embed(&list(&["false"; 4].map(String::from)))),
        ("configData", "\"\"".to_owned()),
        ("warehouseItemList", empty()),
        ("ownedKeyWaordList", numbers_list(&slot.keywords)),
        ("ownedMagicList", empty()),
        ("learnedAbilityList", numbers_list(&learned_ids(slot))),
        ("escapeCount", "0".to_owned()),
        ("battleCount", "42".to_owned()),
        ("corpsSlotIndex", "0".to_owned()),
        ("openChestCount", "17".to_owned()),
        ("ownedMagicStoneList", empty()),
        ("steps", "0".to_owned()),
        ("saveCompleteCount", "3".to_owned()),
        ("monstersKilledCount", "120".to_owned()),
        ("totalGil", "0".to_owned()),
        ("cheatSettingsData", CHEAT.to_owned()),
        ("isOpenedGameBoosterWindow", "false".to_owned()),
    ]);
    object(&pairs, ":")
}

/// Jobs 1 to 3 plus the `jobId` of the fourth seat; an empty seat adds
/// nothing (`docs/formats/pixel-remaster.md#people-and-party-in-ff-ii`).
fn released_jobs(slot: &SlotData) -> Vec<u32> {
    let fourth = slot.corps.get(3).and_then(|id| slot.characters.iter().find(|character| character.id == *id));
    let mut jobs = vec![1, 2, 3];
    jobs.extend(fourth.map(|character| character.job));
    jobs
}

/// Ability 711 first, then the first row of each chain a party member
/// learned, in learning order, without repeat.
fn learned_ids(slot: &SlotData) -> Vec<u32> {
    let mut ids = vec![TWO_SWORD.0];
    for spell in slot.characters.iter().flat_map(|character| &character.spells) {
        let first = super::ff2_save::tables().spell_ability(spell.chain, 1).unwrap();
        if !ids.contains(&first) {
            ids.push(first);
        }
    }
    ids
}

fn ability_object(ability: u32, content: u32, value: i64) -> String {
    format!("{{\"abilityId\":{ability},\"contentId\":{content},\"skillLevel\":{value}}}")
}

fn spell_object(spell: &Spell, value: i64) -> String {
    embed(&ability_object(spell.ability_id(), spell.content_id(), value))
}

fn zeros() -> String {
    embed(&numbers(&(1..=8).map(|level| (level, 0)).collect::<Vec<_>>()))
}

fn render_parameter(character: &Character) -> String {
    let [power, vitality, agility, intelligence, spirit, magic] = character.stats.map(|stat| stat.to_string());
    let zero = || "0".to_owned();
    let pairs: Vec<(&str, String)> = vec![
        ("currentHP", character.hp.to_string()),
        ("currentMP", character.mp.to_string()),
        ("currentMpCountList", zeros()),
        ("addtionalMaxMpCountList", zeros()),
        ("addtionalLevel", "1".to_owned()),
        ("addtionalMaxHp", character.max_hp_gain.to_string()),
        ("addtionalMaxMp", character.max_mp_gain.to_string()),
        ("addtionalPower", power),
        ("addtionalVitality", vitality),
        ("addtionalAgility", agility),
        ("addionalWeight", zero()),
        ("addtionalIntelligence", intelligence),
        ("addtionalSpirit", spirit),
        ("addtionalAttack", zero()),
        ("addtionalDefense", zero()),
        ("addtionalAbilityDefense", zero()),
        ("addtionalAbilityEvasionRate", zero()),
        ("addtionalMagic", magic),
        ("addtionalLuck", zero()),
        ("addtionalAccuracyRate", zero()),
        ("addtionalEvasionRate", zero()),
        ("addtionalAbilityDisturbedRate", zero()),
        ("addtionalCriticalRate", zero()),
        ("addtionalDamageDirmeter", zero()),
        ("addtionalAbilityDefenseRate", zero()),
        ("addtionalAccuracyCount", zero()),
        ("addtionalEvasionCount", zero()),
        ("addtionalDefenseCount", zero()),
        ("addtionalMagicDefenseCount", zero()),
        ("currentConditionList", embed(&list(&[]))),
    ];
    let pairs: Vec<_> = pairs.into_iter().filter(|(key, _)| Some(*key) != character.omit).collect();
    object(&pairs, ":")
}

fn render_character(character: &Character, slot: &SlotData) -> String {
    let equipment_keys: Vec<String> = character.equipment.iter().map(|equip| equip.key.to_string()).collect();
    let equipment_values: Vec<String> = character
        .equipment
        .iter()
        .map(|equip| {
            let mut text = format!("{{\"contentId\":{}", equip.content_id);
            if let Some(count) = &equip.count {
                text.push_str(",\"count\":");
                text.push_str(count);
            }
            text.push('}');
            embed(&text)
        })
        .collect();
    let two_sword = embed(&ability_object(TWO_SWORD.0, TWO_SWORD.1, 0));
    let mut abilities = vec![two_sword];
    abilities.extend(character.spells.iter().map(|spell| spell_object(spell, spell.list_value)));
    let copies: Vec<String> = character.spells.iter().map(|spell| spell_object(spell, spell.dict_value)).collect();
    let magic_command = dict(&["3".to_owned()], &[embed(&list(&copies))]);
    let addition = character
        .addition_order
        .clone()
        .unwrap_or_else(|| std::iter::once(TWO_SWORD.1).chain(character.spells.iter().map(Spell::content_id)).collect());
    let sort = character
        .sort_order
        .clone()
        .unwrap_or_else(|| character.spells.iter().rev().map(Spell::content_id).collect());
    let order = |ids: &[u32]| embed(&list(&ids.iter().map(ToString::to_string).collect::<Vec<_>>()));
    let skill_keys: Vec<String> = character.skills.iter().map(|(key, _)| key.to_string()).collect();
    let skill_values: Vec<String> = character.skills.iter().map(|(_, value)| value.to_string()).collect();
    let slots: Vec<String> = (1..=8)
        .map(|level| {
            let keys = ["1", "2", "3"].map(String::from);
            let empty = vec!["\"\"".to_owned(); 3];
            embed(&format!("{{\"level\":{level},\"slotInfo\":{}}}", embed(&dict(&keys, &empty))))
        })
        .collect();
    let jobs: Vec<String> = std::iter::once(character.job)
        .chain((1..=4).filter(|job| *job != character.job))
        .map(|job| embed(&format!("{{\"id\":{job},\"level\":1,\"currentProficiency\":0}}")))
        .collect();
    let mut pairs: Vec<(&str, String)> = vec![("id", character.id.to_string())];
    if let Some(status) = &character.status_id {
        pairs.push(("characterStatusId", status.clone()));
    }
    pairs.extend([
        ("isEnableCorps", slot.corps.contains(&character.id).to_string()),
        ("jobId", character.job.to_string()),
        ("name", serde_json::Value::String(character.name.clone()).to_string()),
        ("currentExp", (character.id * 900).to_string()),
        ("parameter", embed(&render_parameter(character))),
        ("commandList", embed(&list(&["1", "3", "4", "2", "5", "7"].map(String::from)))),
        ("abilityList", embed(&list(&abilities))),
        ("abilitySlotDataList", embed(&list(&slots))),
        ("jobList", embed(&list(&jobs))),
        ("equipmentList", embed(&dict(&equipment_keys, &equipment_values))),
        ("additionOrderOwnedAbilityIds", order(&addition)),
        ("sortOrderOwnedAbilityIds", order(&sort)),
        ("abilityDictionary", embed(&magic_command)),
        ("skillLevelTargets", embed(&dict(&skill_keys, &skill_values))),
        ("learningAbilitys", embed(&list(&[]))),
        ("equipmentAbilitys", embed(&list(&["0"; 3].map(String::from)))),
        ("numberOfButtles", (character.id % 10).to_string()),
        ("ownedMonsterId", "0".to_owned()),
        ("magicStoneId", "0".to_owned()),
        ("magicLearningValue", "0".to_owned()),
    ]);
    pairs.retain(|(key, _)| Some(*key) != character.omit);
    object(&pairs, ":")
}
