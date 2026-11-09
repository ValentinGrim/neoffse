//! The section names every Pixel Remaster slot uses, the ones that build the
//! first key of a `FieldId`, and the builders of the field lists every game
//! shares.

use neoffse_core::{FieldId, Key};

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
/// Maximum HP shown by the menu, computed by the game. Read only. The `MAX_`
/// cap keys of `bounds.toml` differ per game.
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
/// Luck stat, `parameter.addtionalLuck`. Only FF I declares the field.
pub const LUCK: &str = "luck";
/// Equipment slots, `equipmentList`; the slot is the key that follows.
pub const EQUIPMENT: &str = "equipment";

/// The equipment fields of one character: for each slot 1 to `slots` the
/// pair `equipment.<S>` then `equipment.<S>.count`.
pub(crate) fn equipment_fields(id: u32, slots: u32) -> Vec<FieldId> {
    let mut fields = Vec::with_capacity(2 * slots as usize);
    for slot in 1..=slots {
        let slot_field = FieldId::new(CHARACTER).with_id(id).with_name(EQUIPMENT).with_id(slot);
        fields.push(slot_field.clone());
        fields.push(slot_field.with_name(COUNT));
    }
    fields
}

/// The computed field ids of one character, one per name, in the order given.
pub(crate) fn computed_fields(id: u32, names: [&'static str; 2]) -> Vec<FieldId> {
    names.iter().map(|name| FieldId::new(CHARACTER).with_id(id).with_name(name)).collect()
}

/// `stored` with each computed field placed right after its stored field:
/// `computed` lists `(after, computed)` name pairs, and a stored field whose
/// last key is `after` is followed by `character.<id>.<computed>`.
pub(crate) fn with_computed(stored: Vec<FieldId>, id: u32, computed: &[(&'static str, &'static str)]) -> Vec<FieldId> {
    let mut fields = Vec::with_capacity(stored.len() + computed.len());
    for field in stored {
        let after = match field.keys().last() {
            Some(Key::Name(name)) => computed.iter().find(|(after, _)| after == name).map(|(_, name)| *name),
            _ => None,
        };
        fields.push(field);
        fields.extend(after.map(|name| FieldId::new(CHARACTER).with_id(id).with_name(name)));
    }
    fields
}
