//! The JSON text of a synthetic FF I slot: the nesting of the string
//! documents and the key order of `docs/formats/pixel-remaster.md#slot-keys`
//! and `docs/formats/pixel-remaster.md#characters`.

use super::embed;
use super::ff1_save::{Character, Row, SlotData};

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
    pairs.push(("clearFlag", "1".to_owned()));
    pairs.push(("isCompleteFlag", "1".to_owned()));
    object(&pairs, colon)
}

pub(super) fn user_data(slot: &SlotData) -> String {
    let characters = slot
        .character_list
        .clone()
        .unwrap_or_else(|| list(&slot.characters.iter().map(|character| embed(&render_character(character))).collect::<Vec<_>>()));
    let corps: Vec<String> = slot
        .characters
        .iter()
        .take(4)
        .map(|character| embed(&format!("{{\"id\":1,\"characterId\":{}}}", character.id)))
        .collect();
    let corps_list = list(&corps);
    let rows = |rows: &[Row]| embed(&list(&rows.iter().map(|row| embed(&row.text)).collect::<Vec<_>>()));
    let empty = || embed(&list(&[]));
    let sort_ids: Vec<String> = slot.sort_ids.iter().map(ToString::to_string).collect();
    let vehicle = r#"{"position":{"x":1000.0,"y":1000.0,"z":-1000.0},"direction":0,"id":1,"mapId":-1,"enable":false,"timeStampTicks":0}"#;
    let released: Vec<String> = (1..=12).map(|job| job.to_string()).collect();
    let mut pairs: Vec<(&str, String)> = vec![
        ("corpsList", embed(&corps_list)),
        ("corpsSlots", embed(&dict(&["0", "1", "2"].map(String::from), &vec![embed(&corps_list); 3]))),
        ("ownedCharacterList", embed(&characters)),
        ("releasedJobs", embed(&list(&released))),
    ];
    if let Some(gil) = &slot.gil {
        pairs.push(("owendGil", gil.clone()));
    }
    pairs.extend([
        ("playTime", slot.play_time.clone()),
        ("normalOwnedItemList", rows(&slot.inventory)),
        ("importantOwendItemList", rows(&slot.key_items)),
        ("normalOwnedItemSortIdList", embed(&list(&sort_ids))),
        ("currentArea", "\"Invented Plains\"".to_owned()),
        ("currentLocation", "\"\"".to_owned()),
        ("ownedTransportationList", embed(&list(&vec![embed(vehicle); 4]))),
        ("owendCrystalFlags", embed(&list(&["true"; 4].map(String::from)))),
        ("configData", "\"\"".to_owned()),
        ("warehouseItemList", empty()),
        ("ownedKeyWaordList", empty()),
        ("ownedMagicList", empty()),
        ("learnedAbilityList", empty()),
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

/// `userData.cheatSettingsData` is a plain nested object, not a string
/// (`docs/formats/pixel-remaster.md#documents-inside-strings`).
const CHEAT: &str = r#"{"isEnableEncount":true,"expRate":1,"gilRate":1,"learningProficiencyRate":1,"isHpIncreaseByFightCount":false,"abpRate":1,"magicProficiencyRate":1,"parameterUpRate":1,"magicLearningProficiencyRate":1}"#;

fn object(pairs: &[(&str, String)], colon: &str) -> String {
    let body: Vec<String> = pairs.iter().map(|(key, value)| format!("\"{key}\"{colon}{value}")).collect();
    format!("{{{}}}", body.join(","))
}

fn list(items: &[String]) -> String {
    format!("{{\"target\":[{}]}}", items.join(","))
}

fn dict(keys: &[String], values: &[String]) -> String {
    format!("{{\"keys\":[{}],\"values\":[{}]}}", keys.join(","), values.join(","))
}

fn numbers(entries: &[(u32, u32)]) -> String {
    let keys: Vec<String> = entries.iter().map(|(key, _)| key.to_string()).collect();
    let values: Vec<String> = entries.iter().map(|(_, value)| value.to_string()).collect();
    dict(&keys, &values)
}

fn render_parameter(character: &Character) -> String {
    let [power, vitality, agility, intelligence, spirit, luck, magic] = character.stats.map(|stat| stat.to_string());
    let other = |offset: u32| (character.id * 3 + offset).to_string();
    let pairs: Vec<(&str, String)> = vec![
        ("currentHP", character.hp.to_string()),
        ("currentMP", "0".to_owned()),
        ("currentMpCountList", embed(&numbers(&character.charges))),
        ("addtionalMaxMpCountList", embed(&numbers(&character.charges_gain))),
        ("addtionalLevel", character.level.clone()),
        ("addtionalMaxHp", character.max_hp_gain.to_string()),
        ("addtionalMaxMp", "0".to_owned()),
        ("addtionalPower", power),
        ("addtionalVitality", vitality),
        ("addtionalAgility", agility),
        ("addionalWeight", "0".to_owned()),
        ("addtionalIntelligence", intelligence),
        ("addtionalSpirit", spirit),
        ("addtionalAttack", other(100)),
        ("addtionalDefense", other(110)),
        ("addtionalAbilityDefense", other(120)),
        ("addtionalAbilityEvasionRate", other(130)),
        ("addtionalMagic", magic),
        ("addtionalLuck", luck),
        ("addtionalAccuracyRate", other(140)),
        ("addtionalEvasionRate", other(150)),
        ("addtionalAbilityDisturbedRate", other(160)),
        ("addtionalCriticalRate", other(170)),
        ("addtionalDamageDirmeter", other(180)),
        ("addtionalAbilityDefenseRate", other(190)),
        ("addtionalAccuracyCount", "0".to_owned()),
        ("addtionalEvasionCount", "0".to_owned()),
        ("addtionalDefenseCount", "0".to_owned()),
        ("addtionalMagicDefenseCount", "0".to_owned()),
        ("currentConditionList", embed(&list(&[]))),
    ];
    object(&pairs, ":")
}

fn render_character(character: &Character) -> String {
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
    let abilities: Vec<String> = character
        .abilities
        .iter()
        .map(|(ability, content)| embed(&format!("{{\"abilityId\":{ability},\"contentId\":{content},\"skillLevel\":1}}")))
        .collect();
    let slots: Vec<String> = (1..=8)
        .map(|level| embed(&format!("{{\"level\":{level},\"slotInfo\":{}}}", embed(&dict(&[], &[])))))
        .collect();
    let jobs: Vec<String> = (1..=12)
        .map(|job| embed(&format!("{{\"id\":{job},\"level\":1,\"currentProficiency\":0}}")))
        .collect();
    let pairs: Vec<(&str, String)> = vec![
        ("id", character.id.to_string()),
        ("characterStatusId", character.status_id.to_string()),
        ("isEnableCorps", "true".to_owned()),
        ("jobId", character.job.to_string()),
        ("name", serde_json::Value::String(character.name.clone()).to_string()),
        ("currentExp", character.exp.to_string()),
        ("parameter", embed(&render_parameter(character))),
        ("commandList", embed(&list(&["1", "3", "4", "2", "5"].map(String::from)))),
        ("abilityList", embed(&list(&abilities))),
        ("abilitySlotDataList", embed(&list(&slots))),
        ("jobList", embed(&list(&jobs))),
        ("equipmentList", embed(&dict(&equipment_keys, &equipment_values))),
        ("additionOrderOwnedAbilityIds", embed(&list(&[]))),
        ("sortOrderOwnedAbilityIds", embed(&list(&[]))),
        ("abilityDictionary", embed(&dict(&[], &[]))),
        ("skillLevelTargets", embed(&dict(&[], &[]))),
        ("learningAbilitys", embed(&list(&[]))),
        ("equipmentAbilitys", embed(&list(&["0"; 3].map(String::from)))),
        ("numberOfButtles", "0".to_owned()),
        ("ownedMonsterId", "0".to_owned()),
        ("magicStoneId", "0".to_owned()),
        ("magicLearningValue", "0".to_owned()),
    ];
    object(&pairs, ":")
}

fn config_data() -> String {
    let keys = [
        "buttonType",
        "battleModeIndex",
        "battleSpeedIndex",
        "battleMessageSpeedIndex",
        "isCursorMemory",
        "isKeepAutoBattle",
        "messageSpeed",
        "isLeftActionIcon",
        "isLeftVirtualPad",
        "isLeftMenuCommand",
        "isLeftBattleCommand",
        "reequipIndex",
        "analogModeIndex",
        "isAutoDash",
    ];
    let pairs: Vec<(&str, String)> = keys.iter().map(|key| (*key, "0".to_owned())).collect();
    object(&pairs, ":")
}

fn data_storage() -> String {
    r#"{"scenario":[0,1,4294967295],"treasure":[3,0,255],"global":[0,0],"area":[0],"map":[0],"selected":0,"itemSelected":0,"transportation":[]}"#.to_owned()
}

fn map_data() -> String {
    let player = embed(r#"{"position":{"x":12.0,"y":3.5,"z":0.0},"direction":2}"#);
    let gps = embed(r#"{"mapId":3,"areaId":2,"gpsId":1,"width":256,"height":256}"#);
    format!(
        "{{\"mapId\":3,\"pointIn\":-1,\"transportationId\":1,\"carryingHoverShip\":false,\"playerEntity\":{player},\"companionEntity\":\"\",\"gpsData\":{gps},\"moveCount\":10,\"subtractSteps\":0,\"telepoCacheData\":\"\",\"timerData\":\"\",\"playableCharacterCorpsId\":0,\"encountEnable\":true,\"minimapEnable\":true}}"
    )
}
