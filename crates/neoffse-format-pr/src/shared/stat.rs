//! The seven stats a character gains, shared by the games that store them
//! as `parameter.addtional...` keys. A game builds the variants it has:
//! FF I builds all seven, FF II never builds `Luck`.

use super::names::{AGILITY, INTELLIGENCE, LUCK, MAGIC, POWER, SPIRIT, VITALITY};

/// The seven stat fields; each maps to one `addtional...` key of `parameter`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stat {
    Power,
    Vitality,
    Agility,
    Intelligence,
    Spirit,
    Luck,
    Magic,
}

impl Stat {
    /// The stat whose field name is `name`, `None` for any other name. A game
    /// that lacks a stat refuses the variant itself: FF II drops `Luck`.
    pub(crate) fn of_name(name: &str) -> Option<Self> {
        match name {
            POWER => Some(Stat::Power),
            VITALITY => Some(Stat::Vitality),
            AGILITY => Some(Stat::Agility),
            INTELLIGENCE => Some(Stat::Intelligence),
            SPIRIT => Some(Stat::Spirit),
            LUCK => Some(Stat::Luck),
            MAGIC => Some(Stat::Magic),
            _ => None,
        }
    }

    /// The `parameter` key of the stat (the game's spelling).
    pub(crate) fn document_key(self) -> &'static str {
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
    /// A game whose `bounds.toml` lacks the key has no cap for the stat:
    /// `GameTables::bound` gives `None`, so FF I, which holds no `MAX_MAGIC`,
    /// has no Magic cap.
    pub(crate) fn bound_key(self) -> &'static str {
        match self {
            Stat::Power => "MAX_STRNGTH",
            Stat::Vitality => "MAX_VITALITY",
            Stat::Agility => "MAX_AGILITY",
            Stat::Intelligence => "MAX_INTELIGENCE",
            Stat::Spirit => "MAX_SPRIT",
            Stat::Luck => "MAX_LUCK",
            Stat::Magic => "MAX_MAGIC",
        }
    }

    /// The label of the stat's field.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Stat::Power => "Power",
            Stat::Vitality => "Vitality",
            Stat::Agility => "Agility",
            Stat::Intelligence => "Intelligence",
            Stat::Spirit => "Spirit",
            Stat::Luck => "Luck",
            Stat::Magic => "Magic",
        }
    }
}
