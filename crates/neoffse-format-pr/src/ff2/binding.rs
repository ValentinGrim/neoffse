//! The character fields of an FF II slot. The shared
//! [`Binding`](crate::shared::bind::Binding) turns a [`FieldId`] into gil, a
//! part of a character or the count of an item row, and calls `Part` for the
//! character vocabulary.

use neoffse_core::{EditError, FieldId, FieldMeta, Key, Value};
use neoffse_data::GameTables;

use super::places::Place;
use super::{derived, locate, meta, names, spell};
use crate::document::Document;
use crate::shared::bind::CharacterPart;
use crate::shared::paths::Path;
use crate::shared::read::Located;
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

/// What a [`FieldId`] names in an FF II slot.
pub(super) type Binding = crate::shared::bind::Binding<Part>;

impl CharacterPart for Part {
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

    fn meta(self, tables: &GameTables) -> FieldMeta {
        meta::part_meta(self, tables)
    }

    fn locate(self, document: &mut Document, base: Path) -> Result<Path, Located> {
        locate::locate_part(document, base, self)
    }

    fn is_text(self) -> bool {
        self == Self::Name
    }

    fn is_pathless(self) -> bool {
        matches!(self, Self::MaxHp | Self::MaxMp | Self::Spell(_))
    }

    fn is_locked(self) -> bool {
        matches!(self, Self::SlotCount(_) | Self::MaxHp | Self::MaxMp | Self::Job | Self::Place(_))
    }

    fn computed(self, document: &mut Document, tables: &GameTables, _id: u32, field: &FieldId, base: Path) -> Option<Result<Value, EditError>> {
        match self {
            Self::Spell(chain) => Some(spell::level(document, tables, &base, chain, field)),
            Self::MaxHp | Self::MaxMp => Some(derived::value(document, tables, self, field, base)),
            _ => None,
        }
    }
}
