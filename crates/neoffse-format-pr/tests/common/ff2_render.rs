//! The JSON text of a synthetic FF II slot: the nesting of the string
//! documents and the key order of `docs/formats/pixel-remaster.md#slot-keys`
//! and `docs/formats/pixel-remaster.md#characters`. FF II shares FF I's key
//! set (`docs/formats/pixel-remaster.md#telling-the-games-apart`). The
//! ability keys follow `docs/formats/pixel-remaster.md#abilities` and
//! `docs/formats/pixel-remaster.md#spells-in-ff-ii`.

use super::embed;
use super::ff2_save::{Character, SlotData, Spell};
use super::slot_render::{CharacterParts, UserParts, character_text, dict, list, numbers, object, top_level, user_data_text};

/// Ability 711, first in every `abilityList`: the two-sword permission, with
/// content row 937 (`docs/formats/pixel-remaster.md#nameless-rows-in-ff-ii`).
const TWO_SWORD: (u32, u32) = (711, 937);

/// The whole slot, top level included.
pub(super) fn slot(slot: &SlotData) -> String {
    top_level(slot.spaced, slot.with_user_data.then(|| user_data(slot)), "0")
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
    user_data_text(&UserParts {
        corps: &corps,
        characters,
        released_jobs: &released_jobs(slot),
        gil: slot.gil.as_deref(),
        play_time: &slot.play_time,
        inventory: &slot.inventory,
        key_items: &slot.key_items,
        sort_ids: &slot.sort_ids,
        vehicles: slot.vehicles,
        crystals: "false",
        keywords: &slot.keywords,
        learned: &learned_ids(slot),
    })
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
    character_text(&CharacterParts {
        id: character.id,
        status_id: character.status_id.as_deref(),
        corps: slot.corps.contains(&character.id),
        job: character.job,
        name: &character.name,
        exp: character.id * 900,
        parameter: render_parameter(character),
        commands: &["1", "3", "4", "2", "5", "7"],
        abilities: embed(&list(&abilities)),
        slots: embed(&list(&slots)),
        jobs: embed(&list(&jobs)),
        equipment: &character.equipment,
        addition: order(&addition),
        sort: order(&sort),
        dictionary: embed(&magic_command),
        skills: embed(&dict(&skill_keys, &skill_values)),
        battles: (character.id % 10).to_string(),
        omit: character.omit,
    })
}
