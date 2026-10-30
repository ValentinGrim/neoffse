//! From a [`FieldId`] to what it names: gil, a part of a character, or the
//! count of an item row. [`locate`](super::locate) turns a binding into a
//! path into the document.

use neoffse_core::{EditError, FieldId, Key};

use super::names;
use crate::shared::paths::List;

/// The seven stat fields; each maps to one `addtional...` key of `parameter`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stat {
    Power,
    Vitality,
    Agility,
    Intelligence,
    Spirit,
    Luck,
    Magic,
}

/// A field of one character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Part {
    Name,
    Job,
    Level,
    Exp,
    Hp,
    MaxHpGain,
    /// `addtionalAccuracyRate`.
    AccuracyGain,
    /// `addtionalAbilityEvasionRate`.
    MagicEvasionGain,
    /// Computed from the status table, read only.
    MaxHp,
    /// Computed from the experience table, read only.
    NextLevel,
    Stat(Stat),
    /// `currentMpCountList`, spell level 1 to 8.
    Charges(u32),
    /// `addtionalMaxMpCountList`, spell level 1 to 8.
    ChargesMaxGain(u32),
    /// `contentId` of an equipment slot, 1 to 5.
    Slot(u32),
    /// `count` of an equipment slot, 1 to 5.
    SlotCount(u32),
    /// The ability in a spell slot, read only.
    Spell {
        level: u32,
        slot: u32,
    },
}

/// What a [`FieldId`] names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Binding {
    Gil,
    Character { id: u32, part: Part },
    Count { list: List, key: u32 },
}

impl Binding {
    /// Resolves `field`, or `UnknownField` when it matches no declared field.
    pub(super) fn resolve(field: &FieldId) -> Result<Self, EditError> {
        Self::declared(field.keys()).ok_or_else(|| EditError::UnknownField { field: field.clone() })
    }

    fn declared(keys: &[Key]) -> Option<Self> {
        match keys {
            [Key::Name(names::GIL)] => Some(Self::Gil),
            [Key::Name(names::CHARACTER), Key::Id(id), rest @ ..] => Some(Self::Character {
                id: *id,
                part: Part::declared(rest)?,
            }),
            [Key::Name(section), Key::Id(key), Key::Name(names::COUNT)] => {
                let list = match *section {
                    names::INVENTORY => List::Inventory,
                    names::KEY_ITEMS => List::KeyItems,
                    _ => return None,
                };
                Some(Self::Count { list, key: *key })
            }
            _ => None,
        }
    }

    /// True for the ability id of a spell slot, a value that sits inside a
    /// JSON string.
    pub(super) fn is_spell(&self) -> bool {
        matches!(self, Self::Character { part: Part::Spell { .. }, .. })
    }

    /// True for a computed field, which has no document path.
    pub(super) fn is_derived(&self) -> bool {
        matches!(
            self,
            Self::Character {
                part: Part::MaxHp | Part::NextLevel,
                ..
            }
        )
    }

    /// True for the one text field, `name`.
    pub(super) fn is_text(&self) -> bool {
        matches!(self, Self::Character { part: Part::Name, .. })
    }
}

impl Part {
    fn declared(keys: &[Key]) -> Option<Self> {
        let level_ok = |level: u32| (1..=names::SPELL_LEVELS).contains(&level);
        let slot_ok = |slot: u32| (1..=names::EQUIPMENT_SLOTS).contains(&slot);
        let spell_slot_ok = |slot: u32| (1..=names::SPELL_SLOTS).contains(&slot);
        match keys {
            [Key::Name(names::NAME)] => Some(Self::Name),
            [Key::Name(names::JOB)] => Some(Self::Job),
            [Key::Name(names::LEVEL)] => Some(Self::Level),
            [Key::Name(names::EXP)] => Some(Self::Exp),
            [Key::Name(names::HP)] => Some(Self::Hp),
            [Key::Name(names::MAX_HP_GAIN)] => Some(Self::MaxHpGain),
            [Key::Name(names::ACCURACY_GAIN)] => Some(Self::AccuracyGain),
            [Key::Name(names::MAGIC_EVASION_GAIN)] => Some(Self::MagicEvasionGain),
            [Key::Name(names::MAX_HP)] => Some(Self::MaxHp),
            [Key::Name(names::NEXT_LEVEL)] => Some(Self::NextLevel),
            [Key::Name(names::POWER)] => Some(Self::Stat(Stat::Power)),
            [Key::Name(names::VITALITY)] => Some(Self::Stat(Stat::Vitality)),
            [Key::Name(names::AGILITY)] => Some(Self::Stat(Stat::Agility)),
            [Key::Name(names::INTELLIGENCE)] => Some(Self::Stat(Stat::Intelligence)),
            [Key::Name(names::SPIRIT)] => Some(Self::Stat(Stat::Spirit)),
            [Key::Name(names::LUCK)] => Some(Self::Stat(Stat::Luck)),
            [Key::Name(names::MAGIC)] => Some(Self::Stat(Stat::Magic)),
            [Key::Name(names::CHARGES), Key::Id(level)] if level_ok(*level) => Some(Self::Charges(*level)),
            [Key::Name(names::CHARGES_MAX_GAIN), Key::Id(level)] if level_ok(*level) => Some(Self::ChargesMaxGain(*level)),
            [Key::Name(names::EQUIPMENT), Key::Id(slot)] if slot_ok(*slot) => Some(Self::Slot(*slot)),
            [Key::Name(names::EQUIPMENT), Key::Id(slot), Key::Name(names::COUNT)] if slot_ok(*slot) => Some(Self::SlotCount(*slot)),
            [Key::Name(names::SPELL), Key::Id(level), Key::Id(slot)] if level_ok(*level) && spell_slot_ok(*slot) => {
                Some(Self::Spell { level: *level, slot: *slot })
            }
            _ => None,
        }
    }
}

impl Stat {
    /// The `parameter` key of the stat (the game's spelling).
    pub(super) fn document_key(self) -> &'static str {
        match self {
            Stat::Power => "addtionalPower",
            Stat::Vitality => "addtionalVitality",
            Stat::Agility => "addtionalAgility",
            Stat::Intelligence => "addtionalIntelligence",
            Stat::Spirit => "addtionalSpirit",
            Stat::Luck => "addtionalLuck",
            Stat::Magic => "addtionalMagic",
        }
    }

    /// The `bounds.toml` key of the stat's game cap, spelled as the game
    /// spells it (`docs/formats/pixel-remaster.md#bounds-in-the-master-data`).
    /// Magic has no cap.
    pub(super) fn bound_key(self) -> Option<&'static str> {
        match self {
            Stat::Power => Some("MAX_STRNGTH"),
            Stat::Vitality => Some("MAX_VITALITY"),
            Stat::Agility => Some("MAX_AGILITY"),
            Stat::Intelligence => Some("MAX_INTELIGENCE"),
            Stat::Spirit => Some("MAX_SPRIT"),
            Stat::Luck => Some("MAX_LUCK"),
            Stat::Magic => None,
        }
    }
}
