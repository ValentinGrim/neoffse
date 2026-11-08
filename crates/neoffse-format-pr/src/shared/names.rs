//! The section names every Pixel Remaster slot uses, the ones that build the
//! first key of a `FieldId`.

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
/// Count of an inventory row or of an equipment slot.
pub const COUNT: &str = "count";
/// Character job, `jobId`.
pub const JOB: &str = "job";
/// Current HP, `parameter.currentHP`.
pub const HP: &str = "hp";
/// Maximum HP gain, `parameter.addtionalMaxHp`.
pub const MAX_HP_GAIN: &str = "max_hp_gain";
/// Maximum HP shown by the menu, computed in FF I and FF II. Read only. The
/// `MAX_` cap keys of `bounds.toml` differ per game.
pub const MAX_HP: &str = "max_hp";
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
/// Magic stat, `parameter.addtionalMagic`.
pub const MAGIC: &str = "magic";
/// Equipment slots, `equipmentList`; the slot is the key that follows.
pub const EQUIPMENT: &str = "equipment";
/// `equipment` takes the slot 1 to this value
/// (`docs/formats/pixel-remaster.md#equipment-slots`: keys 1 to 5).
pub const EQUIPMENT_SLOTS: u32 = 5;
