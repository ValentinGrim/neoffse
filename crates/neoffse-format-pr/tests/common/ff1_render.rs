//! The JSON text of a synthetic FF I slot: the nesting of the string
//! documents and the key order of `docs/formats/pixel-remaster.md#slot-keys`
//! and `docs/formats/pixel-remaster.md#characters`. The ability keys follow
//! the measured FF I rules of `docs/formats/pixel-remaster.md#abilities`.

use neoffse_format_pr::ff1::GAME;

use super::embed;
use super::ff1_save::{Character, SlotData};
use super::slot_render::{CharacterParts, UserParts, character_text, dict, list, numbers, object, top_level, user_data_text};

/// The whole slot, top level included.
pub(super) fn slot(slot: &SlotData) -> String {
    top_level(slot.spaced, slot.with_user_data.then(|| user_data(slot)), "1")
}

fn user_data(slot: &SlotData) -> String {
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
    user_data_text(&UserParts {
        corps: &corps,
        characters,
        released_jobs: &(1..=12).collect::<Vec<_>>(),
        gil: slot.gil.as_deref(),
        play_time: &slot.play_time,
        inventory: &slot.inventory,
        key_items: &slot.key_items,
        sort_ids: &slot.sort_ids,
        vehicles: 4,
        crystals: "true",
        keywords: &[],
        learned: &learned_ids(slot),
    })
}

/// The union of the party's `abilityId`s, ascending
/// (`docs/formats/pixel-remaster.md#abilities`: `learnedAbilityList`).
fn learned_ids(slot: &SlotData) -> Vec<u32> {
    let mut ids: Vec<u32> = slot
        .characters
        .iter()
        .flat_map(|character| character.abilities.iter().map(|(id, _)| *id))
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

/// The spell level of an ability, from the ability table.
fn spell_level(content_id: u32) -> u32 {
    let tables = neoffse_data::tables_for(GAME).expect("the FF I tables are embedded");
    let info = tables.ability_by_content(content_id).and_then(|entry| entry.ability);
    u32::from(info.expect("a fixture spell is in the ability table").level)
}

/// The eight `abilitySlotDataList` documents: three slots per spell level,
/// each holding `""` or the ability object of a spell of that level, in
/// the order of `abilityList`.
fn spell_slots(character: &Character) -> Vec<String> {
    (1..=8)
        .map(|level| {
            let mut held = character
                .abilities
                .iter()
                .filter(|(_, content)| spell_level(*content) == level)
                .map(|(ability, content)| embed(&ability_object(*ability, *content)));
            let values: Vec<String> = (0..3).map(|_| held.next().unwrap_or_else(|| "\"\"".to_owned())).collect();
            assert!(held.next().is_none(), "more than three spells at level {level}");
            let keys = ["1", "2", "3"].map(String::from);
            embed(&format!("{{\"level\":{level},\"slotInfo\":{}}}", embed(&dict(&keys, &values))))
        })
        .collect()
}

/// `skillLevel` is 0 in FF I (`docs/formats/pixel-remaster.md#abilities`).
fn ability_object(ability: u32, content: u32) -> String {
    format!("{{\"abilityId\":{ability},\"contentId\":{content},\"skillLevel\":0}}")
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
    let pairs: Vec<_> = pairs.into_iter().filter(|(key, _)| Some(*key) != character.omit).collect();
    object(&pairs, ":")
}

fn render_character(character: &Character) -> String {
    let abilities: Vec<String> = character
        .abilities
        .iter()
        .map(|(ability, content)| embed(&ability_object(*ability, *content)))
        .collect();
    let slots = spell_slots(character);
    let owned: Vec<String> = character.abilities.iter().map(|(_, content)| content.to_string()).collect();
    let jobs: Vec<String> = (1..=12)
        .map(|job| embed(&format!("{{\"id\":{job},\"level\":1,\"currentProficiency\":0}}")))
        .collect();
    character_text(&CharacterParts {
        id: character.id,
        status_id: character.status_id.as_deref(),
        corps: true,
        job: character.job,
        name: &character.name,
        exp: character.exp,
        parameter: render_parameter(character),
        commands: &["1", "3", "4", "2", "5"],
        abilities: embed(&list(&abilities)),
        slots: embed(&list(&slots)),
        jobs: embed(&list(&jobs)),
        equipment: &character.equipment,
        addition: embed(&list(&owned)),
        sort: embed(&list(&[])),
        dictionary: embed(&dict(&[], &[])),
        skills: embed(&dict(&[], &[])),
        battles: "0".to_owned(),
        omit: character.omit,
    })
}
