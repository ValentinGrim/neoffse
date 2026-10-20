//! The field vocabulary of an FF I Pixel Remaster slot: the names that
//! build a [`FieldId`], and the list of the fields of one character.
//!
//! A caller builds an id with the core builders, for example
//! `FieldId::new(names::CHARACTER).with_id(1).with_name(names::LEVEL)`. The
//! document location behind each name is in
//! `docs/formats/pixel-remaster.md#characters`,
//! `docs/formats/pixel-remaster.md#equipment-slots` and
//! `docs/formats/pixel-remaster.md#inventory`.

use neoffse_core::FieldId;

/// Section of the gil counter, `userData.owendGil`.
pub const GIL: &str = "gil";
/// Section of the party characters, one row per `ownedCharacterList` element.
pub const CHARACTER: &str = "character";
/// Section of the normal item rows (items and equipment), `normalOwnedItemList`.
pub const INVENTORY: &str = "inventory";
/// Section of the key item rows, `importantOwendItemList`.
pub const KEY_ITEMS: &str = "key_items";

/// Character name, `name`.
pub const NAME: &str = "name";
/// Character job, `jobId`.
pub const JOB: &str = "job";
/// Character level, `parameter.addtionalLevel`.
pub const LEVEL: &str = "level";
/// Character experience, `currentExp`.
pub const EXP: &str = "exp";
/// Current HP, `parameter.currentHP`.
pub const HP: &str = "hp";
/// Maximum HP gain, `parameter.addtionalMaxHp`.
pub const MAX_HP_GAIN: &str = "max_hp_gain";
/// Power stat, `parameter.addtionalPower`.
pub const POWER: &str = "power";
/// Vitality stat, `parameter.addtionalVitality`.
pub const VITALITY: &str = "vitality";
/// Agility stat, `parameter.addtionalAgility`.
pub const AGILITY: &str = "agility";
/// Intelligence stat, `parameter.addtionalIntelligence`.
pub const INTELLIGENCE: &str = "intelligence";
/// Spirit stat, `parameter.addtionalSpirit`.
pub const SPIRIT: &str = "spirit";
/// Luck stat, `parameter.addtionalLuck`.
pub const LUCK: &str = "luck";
/// Magic stat, `parameter.addtionalMagic`.
pub const MAGIC: &str = "magic";
/// Current spell charges per spell level, `parameter.currentMpCountList`.
pub const CHARGES: &str = "charges";
/// Maximum spell charges gain per spell level, `parameter.addtionalMaxMpCountList`.
pub const CHARGES_MAX_GAIN: &str = "charges_max_gain";
/// Equipment slots, `equipmentList`; the slot is the key that follows.
pub const EQUIPMENT: &str = "equipment";
/// Count of an inventory row or of an equipment slot.
pub const COUNT: &str = "count";

/// `charges` and `charges_max_gain` take the spell level 1 to this value
/// (`docs/formats/pixel-remaster.md#characters`: keys 1 to 8).
pub const SPELL_LEVELS: u32 = 8;
/// `equipment` takes the slot 1 to this value
/// (`docs/formats/pixel-remaster.md#equipment-slots`: keys 1 to 5).
pub const EQUIPMENT_SLOTS: u32 = 5;

/// The scalar fields of a character, in display order.
const SCALARS: [&str; 13] = [
    NAME,
    JOB,
    LEVEL,
    EXP,
    HP,
    MAX_HP_GAIN,
    POWER,
    VITALITY,
    AGILITY,
    INTELLIGENCE,
    SPIRIT,
    LUCK,
    MAGIC,
];

/// The 39 field ids of one character, in display order: the 13 scalar
/// fields, `charges.1` to `charges.8`, `charges_max_gain.1` to
/// `charges_max_gain.8`, then for each slot 1 to 5 the pair `equipment.<S>`
/// and `equipment.<S>.count`.
#[must_use]
pub fn character_fields(id: u32) -> Vec<FieldId> {
    let character = || FieldId::new(CHARACTER).with_id(id);
    let mut fields: Vec<FieldId> = SCALARS.iter().map(|name| character().with_name(name)).collect();
    for name in [CHARGES, CHARGES_MAX_GAIN] {
        fields.extend((1..=SPELL_LEVELS).map(|level| character().with_name(name).with_id(level)));
    }
    for slot in 1..=EQUIPMENT_SLOTS {
        let slot_field = character().with_name(EQUIPMENT).with_id(slot);
        fields.push(slot_field.clone());
        fields.push(slot_field.with_name(COUNT));
    }
    fields
}
