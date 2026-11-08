//! Builds a synthetic FF II slot from Rust data, in the game's style: compact,
//! raw UTF-8, string documents nested as the game nests them
//! (`docs/formats/pixel-remaster.md#documents-inside-strings`).
//!
//! Every value is invented and obeys the game's rules: each equipped id sits
//! in the inventory with a count at least its wearers, each slot `count`
//! equals the inventory count of its id, the five placeholders open the
//! inventory, key items hold `count` 1, every spell entry carries the row of
//! its chain and the proficiency total of that level in both copies, and
//! every gain keeps its base plus gain under the cap. The public fields let a
//! test twist one value; numbers a test breaks are raw text or `i64`.

use neoffse_core::TableKind;
use neoffse_data::GameTables;
use neoffse_format_pr::document::Document;
use neoffse_format_pr::ff2::{Ff2Model, GAME};

pub(crate) use super::ff1_save::Row;

/// The FF II tables.
pub(crate) fn tables() -> &'static GameTables {
    neoffse_data::tables_for(GAME).unwrap()
}

/// One entry of `equipmentList`.
#[derive(Debug, Clone)]
pub(crate) struct Equip {
    pub key: u32,
    /// Raw number text.
    pub content_id: String,
    /// Raw number text; `None` leaves the key out.
    pub count: Option<String>,
}

/// One spell of `abilityList` and of the `abilityDictionary` copy.
#[derive(Debug, Clone)]
pub(crate) struct Spell {
    /// The chain, 0 to 39.
    pub chain: u32,
    /// The level of the row the entry holds, 1 to 16.
    pub row_level: u32,
    /// `skillLevel` of the `abilityList` entry.
    pub list_value: i64,
    /// `skillLevel` of the dictionary entry.
    pub dict_value: i64,
}

impl Spell {
    /// A spell whose row and both values agree.
    pub(crate) fn consistent(chain: u32, level: u32) -> Self {
        let total = i64::from(tables().skill_total(level).unwrap());
        Self {
            chain,
            row_level: level,
            list_value: total,
            dict_value: total,
        }
    }

    pub(crate) fn ability_id(&self) -> u32 {
        tables().spell_ability(self.chain, self.row_level).unwrap()
    }

    pub(crate) fn content_id(&self) -> u32 {
        content_of(self.ability_id())
    }
}

/// The content id of an ability row.
pub(crate) fn content_of(ability: u32) -> u32 {
    tables()
        .entry(TableKind::Ability, ability)
        .and_then(|entry| entry.ability)
        .and_then(|info| info.content_id)
        .unwrap()
}

#[derive(Debug, Clone)]
pub(crate) struct Character {
    pub id: u32,
    /// Raw number text of `characterStatusId`; `None` leaves the key out.
    pub status_id: Option<String>,
    pub job: u32,
    pub name: String,
    pub hp: i64,
    pub mp: i64,
    pub max_hp_gain: i64,
    pub max_mp_gain: i64,
    /// Power, vitality, agility, intelligence, spirit, magic gains.
    pub stats: [i64; 6],
    /// Entries of `skillLevelTargets` in stored order: key, value.
    pub skills: Vec<(u32, i64)>,
    /// Entries of `equipmentList` in stored order.
    pub equipment: Vec<Equip>,
    /// The spells after ability 711, in `abilityList` order.
    pub spells: Vec<Spell>,
    /// `additionOrderOwnedAbilityIds`; `None` lists the content id of every
    /// `abilityList` entry, 711 included.
    pub addition_order: Option<Vec<u32>>,
    /// `sortOrderOwnedAbilityIds`; `None` lists the spells in reverse order.
    pub sort_order: Option<Vec<u32>>,
    /// A key of the character or of its `parameter` to leave out, by name.
    pub omit: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub(crate) struct SlotData {
    /// A space after each top-level colon, so a rewritten value shows.
    pub spaced: bool,
    /// `false` leaves `userData` out.
    pub with_user_data: bool,
    /// The entries of `ownedTransportationList`: 7 for FF II.
    pub vehicles: usize,
    /// Raw number text of `owendGil`; `None` leaves the key out.
    pub gil: Option<String>,
    /// Raw number text of `userData.playTime`.
    pub play_time: String,
    pub characters: Vec<Character>,
    /// Replaces the document of `ownedCharacterList` as given.
    pub character_list: Option<String>,
    /// The `characterId` of each `corpsList` seat; 0 is an empty seat.
    pub corps: Vec<u32>,
    pub inventory: Vec<Row>,
    pub key_items: Vec<Row>,
    /// The content ids of `ownedKeyWaordList`, in acquisition order.
    pub keywords: Vec<u32>,
    pub sort_ids: Vec<u32>,
}

/// The ids of the non-placeholder rows of a table, ascending.
pub(crate) fn ids(kind: TableKind) -> Vec<u32> {
    tables()
        .table(kind)
        .unwrap()
        .iter()
        .filter(|entry| !entry.placeholder)
        .map(|entry| entry.id)
        .collect()
}

const NAMES: [&str; 4] = ["Ivo", "Mara", "Tane", "Élan"];
const MAX_HP_GAIN: [i64; 4] = [120, 85, 210, 1400];
const MAX_MP_GAIN: [i64; 4] = [14, 9, 6, 80];
const HP: [i64; 4] = [140, 105, 200, 1500];
const MP: [i64; 4] = [20, 19, 16, 150];

/// The inventory of the standard slot: content id, count, in document order.
/// The five placeholders come first with the counts of
/// `docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`.
pub(crate) fn inventory() -> Vec<(u32, u32)> {
    let [weapons, shields, armors, helmets, gloves, items] = [
        TableKind::Weapon,
        TableKind::Shield,
        TableKind::Armor,
        TableKind::Helmet,
        TableKind::Gloves,
        TableKind::Item,
    ]
    .map(ids);
    let mut rows = vec![(47, 10), (208, 5), (209, 5), (210, 11), (211, 11)];
    rows.extend([(items[0], 7), (items[1], 4), (items[2], 2), (items[5], 9), (items[8], 1)]);
    rows.extend([(60, 3), (91, 1), (105, 1), (weapons[2], 1), (weapons[3], 2), (shields[1], 1)]);
    rows.extend([(armors[0], 2), (armors[1], 1), (helmets[0], 2), (helmets[1], 1), (gloves[0], 2), (gloves[1], 1)]);
    rows.extend([(weapons[10], 1), (armors[5], 1), (helmets[5], 1), (gloves[5], 1)]);
    rows
}

/// The key items of the standard slot, `count` 1 each.
pub(crate) fn key_items() -> Vec<u32> {
    let all = ids(TableKind::KeyItem);
    [0, 1, 2, 4, 5, 7, 8, 12].map(|index| all[index]).to_vec()
}

/// The keywords of the standard slot, in acquisition order.
pub(crate) fn keywords() -> Vec<u32> {
    let all = ids(TableKind::Keyword);
    [0, 1, 4, 2].map(|index| all[index]).to_vec()
}

/// The equipment of the standard characters, slot 1 to 5. Ids 47 and 208 to
/// 211 are the placeholders; 60, 91 and 105 are rows the document names.
fn worn(index: usize) -> [u32; 5] {
    let [weapons, shields, armors, helmets, gloves] = [TableKind::Weapon, TableKind::Shield, TableKind::Armor, TableKind::Helmet, TableKind::Gloves].map(ids);
    match index {
        0 => [60, 60, armors[0], helmets[0], gloves[0]],
        1 => [91, 105, armors[1], 210, gloves[1]],
        2 => [weapons[2], 47, 209, helmets[1], 211],
        _ => [weapons[3], shields[1], armors[0], helmets[0], gloves[0]],
    }
}

fn spells(index: usize) -> Vec<Spell> {
    match index {
        0 => vec![Spell::consistent(0, 5), Spell::consistent(1, 1), Spell::consistent(2, 16)],
        1 => {
            let mismatched = Spell {
                chain: 5,
                row_level: 1,
                list_value: 456,
                dict_value: 400,
            };
            vec![Spell::consistent(3, 2), Spell::consistent(4, 8), mismatched, Spell::consistent(6, 11)]
        }
        2 => vec![Spell::consistent(7, 3)],
        _ => (0..16).map(|chain| Spell::consistent(chain, chain + 1)).collect(),
    }
}

fn standard_character(index: usize) -> Character {
    let id = u32::try_from(index).unwrap() + 1;
    let count_of = |content: u32| inventory().iter().find(|(row, _)| *row == content).map_or(0, |(_, count)| *count);
    let key = i64::from(id);
    Character {
        id,
        status_id: Some(id.to_string()),
        job: id,
        name: NAMES[index].to_owned(),
        hp: HP[index],
        mp: MP[index],
        max_hp_gain: MAX_HP_GAIN[index],
        max_mp_gain: MAX_MP_GAIN[index],
        stats: [0, 1, 2, 3, 4, 5].map(|stat| key * 7 + stat * 3),
        skills: (0..10)
            .map(|skill| {
                (
                    skill,
                    if skill < 8 {
                        (key * 37 + i64::from(skill) * 53) % 700
                    } else {
                        key * 100 + i64::from(skill)
                    },
                )
            })
            .collect(),
        equipment: (1..=5)
            .zip(worn(index))
            .map(|(key, content)| Equip {
                key,
                content_id: content.to_string(),
                count: Some(count_of(content).to_string()),
            })
            .collect(),
        spells: spells(index),
        addition_order: None,
        sort_order: None,
        omit: None,
    }
}

impl SlotData {
    /// The standard slot: four characters, three of them in the party, the
    /// rules of the module docs.
    pub(crate) fn standard() -> Self {
        Self {
            spaced: false,
            with_user_data: true,
            vehicles: 7,
            gil: Some("1234567".to_owned()),
            play_time: "13621.814333377138".to_owned(),
            characters: (0..4).map(standard_character).collect(),
            character_list: None,
            corps: vec![1, 2, 3, 0],
            inventory: inventory().iter().map(|(id, count)| Row::new(*id, *count)).collect(),
            key_items: key_items().iter().map(|id| Row::new(*id, 1)).collect(),
            keywords: keywords(),
            sort_ids: ids(TableKind::Item).into_iter().take(8).collect(),
        }
    }

    /// The slot as JSON text.
    pub(crate) fn render(&self) -> String {
        super::ff2_render::slot(self)
    }
}

/// A model over `text`, with the FF II tables.
pub(crate) fn model(text: &str) -> Ff2Model {
    Ff2Model::new(Document::parse(text).unwrap(), tables())
}

/// A model over the standard slot.
pub(crate) fn standard_model() -> Ff2Model {
    model(&SlotData::standard().render())
}
