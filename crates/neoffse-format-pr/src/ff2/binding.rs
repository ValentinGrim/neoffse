//! From a [`FieldId`] to what it names: gil, a part of a character, or the
//! count of an item row. [`locate`](super::locate) turns a binding into a
//! path into the document.

use neoffse_core::{EditError, FieldId, Key};

use super::names;
use super::places::Place;
use crate::shared::paths::List;
use crate::shared::stat::Stat;

/// A field of one character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Part {
    Name,
    /// `jobId`: the person (`docs/formats/pixel-remaster.md#people-and-party-in-ff-ii`).
    Job,
    Hp,
    Mp,
    MaxHpGain,
    MaxMpGain,
    /// Computed from the status table, read only.
    MaxHp,
    /// Computed from the status table, read only.
    MaxMp,
    /// One of the six stat gains; FF II never builds `Stat::Luck`.
    Stat(Stat),
    /// `skillLevelTargets`, key 0 to 9.
    Skill(u32),
    /// The level of the spell of one chain, read from the row of its
    /// `abilityList` entry. It has no document path of its own.
    Spell(u32),
    /// `contentId` of an equipment slot, 1 to 5.
    Slot(u32),
    /// `count` of an equipment slot, 1 to 5.
    SlotCount(u32),
    /// A stored place of a spell, by position in its list (expert pages).
    Place(Place),
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
            _ => List::of_count_field(keys).map(|(list, key)| Self::Count { list, key }),
        }
    }

    /// True for a computed field, which has no document path.
    pub(super) fn is_derived(&self) -> bool {
        matches!(
            self,
            Self::Character {
                part: Part::MaxHp | Part::MaxMp,
                ..
            }
        )
    }

    /// True for the level of a spell, which reads a row and writes five
    /// places (`docs/formats/pixel-remaster.md#row-and-value-in-ff-ii`).
    pub(super) fn is_spell(&self) -> bool {
        matches!(self, Self::Character { part: Part::Spell(_), .. })
    }

    /// True for the one text field, `name`.
    pub(super) fn is_text(&self) -> bool {
        matches!(self, Self::Character { part: Part::Name, .. })
    }
}

impl Part {
    fn declared(keys: &[Key]) -> Option<Self> {
        let skill_ok = |key: u32| key < names::SKILLS;
        let slot_ok = |slot: u32| (1..=names::EQUIPMENT_SLOTS).contains(&slot);
        match keys {
            [Key::Name(names::NAME)] => Some(Self::Name),
            [Key::Name(names::JOB)] => Some(Self::Job),
            [Key::Name(names::HP)] => Some(Self::Hp),
            [Key::Name(names::MP)] => Some(Self::Mp),
            [Key::Name(names::MAX_HP_GAIN)] => Some(Self::MaxHpGain),
            [Key::Name(names::MAX_MP_GAIN)] => Some(Self::MaxMpGain),
            [Key::Name(names::MAX_HP)] => Some(Self::MaxHp),
            [Key::Name(names::MAX_MP)] => Some(Self::MaxMp),
            [Key::Name(names::POWER)] => Some(Self::Stat(Stat::Power)),
            [Key::Name(names::VITALITY)] => Some(Self::Stat(Stat::Vitality)),
            [Key::Name(names::AGILITY)] => Some(Self::Stat(Stat::Agility)),
            [Key::Name(names::INTELLIGENCE)] => Some(Self::Stat(Stat::Intelligence)),
            [Key::Name(names::SPIRIT)] => Some(Self::Stat(Stat::Spirit)),
            [Key::Name(names::MAGIC)] => Some(Self::Stat(Stat::Magic)),
            [Key::Name(names::SKILL), Key::Id(key)] if skill_ok(*key) => Some(Self::Skill(*key)),
            [Key::Name(names::SPELL), Key::Id(chain)] => Some(Self::Spell(*chain)),
            [Key::Name(names::EQUIPMENT), Key::Id(slot)] if slot_ok(*slot) => Some(Self::Slot(*slot)),
            [Key::Name(names::EQUIPMENT), Key::Id(slot), Key::Name(names::COUNT)] if slot_ok(*slot) => Some(Self::SlotCount(*slot)),
            _ => Place::declared(keys).map(Self::Place),
        }
    }
}
