//! The field vocabulary of an FF II Pixel Remaster slot: the names that
//! build a [`FieldId`], and the lists of the fields of one character.
//!
//! A caller builds an id with the core builders, for example
//! `FieldId::new(names::CHARACTER).with_id(1).with_name(names::HP)`. The
//! document location behind each name is in
//! `docs/formats/pixel-remaster.md#characters`,
//! `docs/formats/pixel-remaster.md#skill-levels-in-ff-ii` and
//! `docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`.
//!
//! FF II has no level, experience or spell charge field: the six stats,
//! the maximum HP and MP and the weapon skills are the stored gains and
//! values of `docs/formats/pixel-remaster.md#character-growth-in-ff-ii`.

use neoffse_core::{FieldId, Key};

pub use crate::shared::names::{
    AGILITY, CHARACTER, COUNT, EQUIPMENT, EQUIPMENT_SLOTS, GIL, HP, INTELLIGENCE, INVENTORY, JOB, KEY_ITEMS, MAGIC, MAX_HP, MAX_HP_GAIN, NAME, POWER, SPIRIT,
    VITALITY,
};

/// Current MP, `parameter.currentMP`.
pub const MP: &str = "mp";
/// Maximum MP shown by the menu, computed. Read only.
pub const MAX_MP: &str = "max_mp";
/// Maximum MP gain, `parameter.addtionalMaxMp`.
pub const MAX_MP_GAIN: &str = "max_mp_gain";
/// One skill value, `skillLevelTargets`; the key 0 to 9 follows
/// (`docs/formats/pixel-remaster.md#skill-levels-in-ff-ii`).
pub const SKILL: &str = "skill";
/// `skill` takes the key 0 to this value minus 1.
pub const SKILLS: u32 = 10;

/// The level of one spell, `character.<id>.spell.<chain>`: the chain, 0 to 39,
/// follows. The value is the level of the row the first `abilityList` entry
/// of the chain holds (`docs/formats/pixel-remaster.md#spells-in-ff-ii`).
pub const SPELL: &str = "spell";

/// The field id of the level of spell `chain` of character `id`.
#[must_use]
pub fn spell_field(id: u32, chain: u32) -> FieldId {
    FieldId::new(CHARACTER).with_id(id).with_name(SPELL).with_id(chain)
}

/// The entity id of the raw spell places of a character,
/// `character.<id>.raw_spells`: the expert page of the Magic section.
pub const RAW_SPELLS: &str = "raw_spells";
/// A place of an `abilityList` entry,
/// `character.<id>.ability.<position>.<part>`. The position is the index in
/// the list, counted from 0, never the ability id: an edit of the id keeps
/// the field (`docs/formats/pixel-remaster.md#abilities`).
pub const ABILITY: &str = "ability";
/// A place of an entry of the Magic command's copy in `abilityDictionary`,
/// `character.<id>.dictionary.<position>.<part>`, addressed like [`ABILITY`].
pub const DICTIONARY: &str = "dictionary";
/// A content id of `additionOrderOwnedAbilityIds`,
/// `character.<id>.addition_order.<position>`.
pub const ADDITION_ORDER: &str = "addition_order";
/// A content id of `sortOrderOwnedAbilityIds`,
/// `character.<id>.sort_order.<position>`.
pub const SORT_ORDER: &str = "sort_order";
/// The part of an entry that holds `abilityId`.
pub const ABILITY_ID: &str = "ability_id";
/// The part of an entry that holds `contentId`.
pub const CONTENT_ID: &str = "content_id";
/// The part of an entry that holds `skillLevel`, the proficiency total
/// (`docs/formats/pixel-remaster.md#skill-levels-in-ff-ii`).
pub const PROFICIENCY: &str = "proficiency";

/// The field id of one part of an entry: `group` is [`ABILITY`] or
/// [`DICTIONARY`], `part` one of [`ABILITY_ID`], [`CONTENT_ID`] and
/// [`PROFICIENCY`].
#[must_use]
pub fn place(id: u32, group: &'static str, position: u32, part: &'static str) -> FieldId {
    FieldId::new(CHARACTER).with_id(id).with_name(group).with_id(position).with_name(part)
}

/// The field id of one position of an order list: `order` is
/// [`ADDITION_ORDER`] or [`SORT_ORDER`].
#[must_use]
pub fn order_place(id: u32, order: &'static str, position: u32) -> FieldId {
    FieldId::new(CHARACTER).with_id(id).with_name(order).with_id(position)
}

/// The stored field ids of one character, in display order: `name`, `job`,
/// `hp`, `max_hp_gain`, `mp`, `max_mp_gain`, the six stat gains, `skill.0` to
/// `skill.9`, then for each slot 1 to 5 the pair `equipment.<S>` and
/// `equipment.<S>.count`. That makes 32 ids.
#[must_use]
pub fn character_fields(id: u32) -> Vec<FieldId> {
    let character = || FieldId::new(CHARACTER).with_id(id);
    let named = |names: &[&'static str]| names.iter().map(|name| character().with_name(name)).collect::<Vec<_>>();
    let mut fields = named(&[
        NAME,
        JOB,
        HP,
        MAX_HP_GAIN,
        MP,
        MAX_MP_GAIN,
        POWER,
        VITALITY,
        AGILITY,
        INTELLIGENCE,
        SPIRIT,
        MAGIC,
    ]);
    fields.extend((0..SKILLS).map(|key| character().with_name(SKILL).with_id(key)));
    for slot in 1..=EQUIPMENT_SLOTS {
        let slot_field = character().with_name(EQUIPMENT).with_id(slot);
        fields.push(slot_field.clone());
        fields.push(slot_field.with_name(COUNT));
    }
    fields
}

/// The two computed field ids of one character: `max_hp`, then `max_mp`.
/// Neither has a document path.
#[must_use]
pub fn derived_fields(id: u32) -> Vec<FieldId> {
    [MAX_HP, MAX_MP]
        .iter()
        .map(|name| FieldId::new(CHARACTER).with_id(id).with_name(name))
        .collect()
}

/// The 34 field ids of the Party entity of one character, in view order:
/// [`character_fields`] with `max_hp` after `hp` and `max_mp` after `mp`.
#[must_use]
pub fn party_fields(id: u32) -> Vec<FieldId> {
    let computed = |name| FieldId::new(CHARACTER).with_id(id).with_name(name);
    let mut fields = Vec::with_capacity(34);
    for field in character_fields(id) {
        let derived = match field.keys().last() {
            Some(Key::Name(HP)) => Some(computed(MAX_HP)),
            Some(Key::Name(MP)) => Some(computed(MAX_MP)),
            _ => None,
        };
        fields.push(field);
        fields.extend(derived);
    }
    fields
}
