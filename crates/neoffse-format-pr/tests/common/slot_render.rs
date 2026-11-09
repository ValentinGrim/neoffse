//! The pieces of a synthetic slot text that FF I and FF II share: the top
//! level, the `userData` and character documents with their key order
//! (`docs/formats/pixel-remaster.md#slot-keys` and
//! `docs/formats/pixel-remaster.md#characters`), the equipment list and the
//! small documents that fill the rest of the slot. A game renderer passes
//! the values that differ.

use super::{VEHICLE, embed};

/// One entry of `equipmentList`.
#[derive(Debug, Clone)]
pub(crate) struct Equip {
    pub key: u32,
    /// Raw number text.
    pub content_id: String,
    /// Raw number text; `None` leaves the key out.
    pub count: Option<String>,
}

impl Equip {
    /// Slots 1 to 5 holding `worn`, each with the count `count_of` gives its id.
    pub(crate) fn worn(worn: [u32; 5], count_of: impl Fn(u32) -> u32) -> Vec<Self> {
        (1..=5)
            .zip(worn)
            .map(|(key, content)| Self {
                key,
                content_id: content.to_string(),
                count: Some(count_of(content).to_string()),
            })
            .collect()
    }
}

/// A row of an item list: the JSON text of the element, so a test can break it.
#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub text: String,
}

impl Row {
    /// The canonical element text of the game.
    pub(crate) fn new(content_id: u32, count: u32) -> Self {
        Self::raw(format!("{{\"contentId\":{content_id},\"count\":{count}}}"))
    }

    pub(crate) fn raw(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

/// The whole slot, top level included. `user_data` is the document of
/// `userData`, `None` leaves the key out; `clear_flag` is the text of
/// `clearFlag` and `isCompleteFlag`.
pub(super) fn top_level(spaced: bool, user_data: Option<String>, clear_flag: &str) -> String {
    let colon = if spaced { ": " } else { ":" };
    let mut pairs: Vec<(&str, String)> = vec![("id", "1".to_owned()), ("pictureData", "\"UE5HLWludmVudGVk\"".to_owned())];
    if let Some(user_data) = user_data {
        pairs.push(("userData", embed(&user_data)));
    }
    pairs.push(("configData", embed(&config_data())));
    pairs.push(("dataStorage", embed(&data_storage())));
    pairs.push(("mapData", embed(&map_data())));
    pairs.push(("timeStamp", "\"3/17/2026 6:42:09 PM\"".to_owned()));
    pairs.push(("playTime", "0.0".to_owned()));
    pairs.push(("clearFlag", clear_flag.to_owned()));
    pairs.push(("isCompleteFlag", clear_flag.to_owned()));
    object(&pairs, colon)
}

/// What differs between the games in the `userData` document. The pair order
/// and every shared value live in [`user_data_text`].
pub(super) struct UserParts<'a> {
    /// The `corpsList` rows, each an embedded document.
    pub corps: &'a [String],
    /// The document of `ownedCharacterList`, not yet embedded.
    pub characters: String,
    pub released_jobs: &'a [u32],
    /// Raw number text; `None` leaves `owendGil` out.
    pub gil: Option<&'a str>,
    pub play_time: &'a str,
    pub inventory: &'a [Row],
    pub key_items: &'a [Row],
    pub sort_ids: &'a [u32],
    /// The entries of `ownedTransportationList`.
    pub vehicles: usize,
    /// `true` or `false`, the four `owendCrystalFlags`.
    pub crystals: &'a str,
    pub keywords: &'a [u32],
    pub learned: &'a [u32],
}

/// The `userData` document of `parts`, in the key order of
/// `docs/formats/pixel-remaster.md#slot-keys`.
pub(super) fn user_data_text(parts: &UserParts<'_>) -> String {
    let corps_list = list(parts.corps);
    let rows = |rows: &[Row]| embed(&list(&rows.iter().map(|row| embed(&row.text)).collect::<Vec<_>>()));
    let empty = || embed(&list(&[]));
    let numbers_list = |ids: &[u32]| embed(&list(&ids.iter().map(ToString::to_string).collect::<Vec<_>>()));
    let mut pairs: Vec<(&str, String)> = vec![
        ("corpsList", embed(&corps_list)),
        ("corpsSlots", embed(&dict(&["0", "1", "2"].map(String::from), &vec![embed(&corps_list); 3]))),
        ("ownedCharacterList", embed(&parts.characters)),
        ("releasedJobs", numbers_list(parts.released_jobs)),
    ];
    if let Some(gil) = parts.gil {
        pairs.push(("owendGil", gil.to_owned()));
    }
    pairs.extend([
        ("playTime", parts.play_time.to_owned()),
        ("normalOwnedItemList", rows(parts.inventory)),
        ("importantOwendItemList", rows(parts.key_items)),
        ("normalOwnedItemSortIdList", numbers_list(parts.sort_ids)),
        ("currentArea", "\"Invented Plains\"".to_owned()),
        ("currentLocation", "\"\"".to_owned()),
        ("ownedTransportationList", embed(&list(&vec![embed(VEHICLE); parts.vehicles]))),
        ("owendCrystalFlags", embed(&list(&[parts.crystals; 4].map(String::from)))),
        ("configData", "\"\"".to_owned()),
        ("warehouseItemList", empty()),
        ("ownedKeyWaordList", numbers_list(parts.keywords)),
        ("ownedMagicList", empty()),
        ("learnedAbilityList", numbers_list(parts.learned)),
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
pub(super) const CHEAT: &str = r#"{"isEnableEncount":true,"expRate":1,"gilRate":1,"learningProficiencyRate":1,"isHpIncreaseByFightCount":false,"abpRate":1,"magicProficiencyRate":1,"parameterUpRate":1,"magicLearningProficiencyRate":1}"#;

pub(super) fn object(pairs: &[(&str, String)], colon: &str) -> String {
    let body: Vec<String> = pairs.iter().map(|(key, value)| format!("\"{key}\"{colon}{value}")).collect();
    format!("{{{}}}", body.join(","))
}

pub(super) fn list(items: &[String]) -> String {
    format!("{{\"target\":[{}]}}", items.join(","))
}

pub(super) fn dict(keys: &[String], values: &[String]) -> String {
    format!("{{\"keys\":[{}],\"values\":[{}]}}", keys.join(","), values.join(","))
}

pub(super) fn numbers(entries: &[(u32, u32)]) -> String {
    let keys: Vec<String> = entries.iter().map(|(key, _)| key.to_string()).collect();
    let values: Vec<String> = entries.iter().map(|(_, value)| value.to_string()).collect();
    dict(&keys, &values)
}

/// What differs between the games in a character document. The pair order
/// and every shared value live in [`character_text`]; a value that is a
/// string document arrives already embedded.
pub(super) struct CharacterParts<'a> {
    pub id: u32,
    /// Raw number text; `None` leaves `characterStatusId` out.
    pub status_id: Option<&'a str>,
    pub corps: bool,
    pub job: u32,
    pub name: &'a str,
    pub exp: u32,
    /// The `parameter` document, not yet embedded.
    pub parameter: String,
    pub commands: &'a [&'a str],
    pub abilities: String,
    pub slots: String,
    pub jobs: String,
    pub equipment: &'a [Equip],
    pub addition: String,
    pub sort: String,
    pub dictionary: String,
    pub skills: String,
    /// Raw number text of `numberOfButtles`.
    pub battles: String,
    pub omit: Option<&'a str>,
}

/// The character document of `parts`, in the key order of
/// `docs/formats/pixel-remaster.md#characters`.
pub(super) fn character_text(parts: &CharacterParts<'_>) -> String {
    let commands: Vec<String> = parts.commands.iter().map(ToString::to_string).collect();
    let mut pairs: Vec<(&str, String)> = vec![("id", parts.id.to_string())];
    if let Some(status) = parts.status_id {
        pairs.push(("characterStatusId", status.to_owned()));
    }
    pairs.extend([
        ("isEnableCorps", parts.corps.to_string()),
        ("jobId", parts.job.to_string()),
        ("name", serde_json::Value::String(parts.name.to_owned()).to_string()),
        ("currentExp", parts.exp.to_string()),
        ("parameter", embed(&parts.parameter)),
        ("commandList", embed(&list(&commands))),
        ("abilityList", parts.abilities.clone()),
        ("abilitySlotDataList", parts.slots.clone()),
        ("jobList", parts.jobs.clone()),
        ("equipmentList", equipment_list(parts.equipment)),
        ("additionOrderOwnedAbilityIds", parts.addition.clone()),
        ("sortOrderOwnedAbilityIds", parts.sort.clone()),
        ("abilityDictionary", parts.dictionary.clone()),
        ("skillLevelTargets", parts.skills.clone()),
        ("learningAbilitys", embed(&list(&[]))),
        ("equipmentAbilitys", embed(&list(&["0"; 3].map(String::from)))),
        ("numberOfButtles", parts.battles.clone()),
        ("ownedMonsterId", "0".to_owned()),
        ("magicStoneId", "0".to_owned()),
        ("magicLearningValue", "0".to_owned()),
    ]);
    pairs.retain(|(key, _)| Some(*key) != parts.omit);
    object(&pairs, ":")
}

/// The embedded `equipmentList` document of `equipment`: slot keys, then one
/// embedded `{contentId, count}` row each, with `count` left out when `None`.
pub(super) fn equipment_list(equipment: &[Equip]) -> String {
    let keys: Vec<String> = equipment.iter().map(|equip| equip.key.to_string()).collect();
    let values: Vec<String> = equipment
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
    embed(&dict(&keys, &values))
}

pub(super) fn config_data() -> String {
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

pub(super) fn data_storage() -> String {
    r#"{"scenario":[0,1,4294967295],"treasure":[3,0,255],"global":[0,0],"area":[0],"map":[0],"selected":0,"itemSelected":0,"transportation":[]}"#.to_owned()
}

pub(super) fn map_data() -> String {
    let player = embed(r#"{"position":{"x":12.0,"y":3.5,"z":0.0},"direction":2}"#);
    let gps = embed(r#"{"mapId":3,"areaId":2,"gpsId":1,"width":256,"height":256}"#);
    format!(
        "{{\"mapId\":3,\"pointIn\":-1,\"transportationId\":1,\"carryingHoverShip\":false,\"playerEntity\":{player},\"companionEntity\":\"\",\"gpsData\":{gps},\"moveCount\":10,\"subtractSteps\":0,\"telepoCacheData\":\"\",\"timerData\":\"\",\"playableCharacterCorpsId\":0,\"encountEnable\":true,\"minimapEnable\":true}}"
    )
}
