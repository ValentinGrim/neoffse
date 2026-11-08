//! The character fields of an FF I slot. The shared
//! [`Binding`](crate::shared::bind::Binding) turns a [`FieldId`] into gil, a
//! part of a character or the count of an item row, and calls `Part` for the
//! character vocabulary.

use neoffse_core::{EditError, FieldId, FieldMeta, Key, Value};
use neoffse_data::GameTables;

use super::{derived, locate, meta, names};
use crate::document::Document;
use crate::shared::bind::CharacterPart;
use crate::shared::paths::Path;
use crate::shared::read::Located;
use crate::shared::stat::Stat;

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

impl CharacterPart for Part {
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
        matches!(self, Self::MaxHp | Self::NextLevel)
    }

    fn is_locked(self) -> bool {
        matches!(self, Self::SlotCount(_) | Self::Spell { .. } | Self::MaxHp | Self::NextLevel)
    }

    fn computed(self, document: &mut Document, tables: &GameTables, _id: u32, field: &FieldId, base: Path) -> Option<Result<Value, EditError>> {
        match self {
            Self::MaxHp | Self::NextLevel => Some(derived::value(document, tables, self, field, base)),
            Self::Spell { .. } => Some(locate::spell_ability(document, self, field, base)),
            _ => None,
        }
    }
}
