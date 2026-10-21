//! From a [`FieldId`] to what it names: gil, a part of a character, or the
//! count of an item row. [`locate`](super::locate) turns a binding into a
//! path into the document.

use neoffse_core::{EditError, FieldId, Key};

use super::names;
use crate::document::Segment;

/// A path into the document; every key name is a `'static` literal.
pub(super) type Path = Vec<Segment<'static>>;

/// The two lists of `userData` that hold `{contentId, count}` rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum List {
    /// `normalOwnedItemList`: items and equipment.
    Inventory,
    /// `importantOwendItemList`: key items.
    KeyItems,
}

impl List {
    /// The document key of the list (the game's spelling).
    fn document_key(self) -> &'static str {
        match self {
            List::Inventory => "normalOwnedItemList",
            List::KeyItems => "importantOwendItemList",
        }
    }

    /// The section name of the list's rows.
    pub(super) fn section(self) -> &'static str {
        match self {
            List::Inventory => names::INVENTORY,
            List::KeyItems => names::KEY_ITEMS,
        }
    }

    /// Path to the `target` array of the list.
    pub(super) fn target(self) -> Path {
        vec![
            Segment::Key("userData"),
            Segment::Doc,
            Segment::Key(self.document_key()),
            Segment::Doc,
            Segment::Key("target"),
        ]
    }
}

/// A collection of the model, named by its section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Collection {
    /// `ownedCharacterList`.
    Character,
    /// One of the item lists.
    Items(List),
}

impl Collection {
    /// The collection that `field` names exactly, if any.
    pub(super) fn of(field: &FieldId) -> Option<Self> {
        match field.keys() {
            [Key::Name(names::CHARACTER)] => Some(Self::Character),
            [Key::Name(names::INVENTORY)] => Some(Self::Items(List::Inventory)),
            [Key::Name(names::KEY_ITEMS)] => Some(Self::Items(List::KeyItems)),
            _ => None,
        }
    }

    /// Path to the `target` array of the collection.
    pub(super) fn target(self) -> Path {
        match self {
            Self::Character => characters_target(),
            Self::Items(list) => list.target(),
        }
    }

    /// The key that identifies a row inside its element.
    pub(super) fn row_key(self) -> &'static str {
        match self {
            Self::Character => "id",
            Self::Items(_) => "contentId",
        }
    }
}

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
    Stat(Stat),
    /// `currentMpCountList`, spell level 1 to 8.
    Charges(u32),
    /// `addtionalMaxMpCountList`, spell level 1 to 8.
    ChargesMaxGain(u32),
    /// `contentId` of an equipment slot, 1 to 5.
    Slot(u32),
    /// `count` of an equipment slot, 1 to 5.
    SlotCount(u32),
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

    /// True for the one text field, `name`.
    pub(super) fn is_text(&self) -> bool {
        matches!(self, Self::Character { part: Part::Name, .. })
    }
}

impl Part {
    fn declared(keys: &[Key]) -> Option<Self> {
        let level_ok = |level: u32| (1..=names::SPELL_LEVELS).contains(&level);
        let slot_ok = |slot: u32| (1..=names::EQUIPMENT_SLOTS).contains(&slot);
        match keys {
            [Key::Name(names::NAME)] => Some(Self::Name),
            [Key::Name(names::JOB)] => Some(Self::Job),
            [Key::Name(names::LEVEL)] => Some(Self::Level),
            [Key::Name(names::EXP)] => Some(Self::Exp),
            [Key::Name(names::HP)] => Some(Self::Hp),
            [Key::Name(names::MAX_HP_GAIN)] => Some(Self::MaxHpGain),
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

/// Path to `ownedCharacterList.target`.
pub(super) fn characters_target() -> Path {
    vec![
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("ownedCharacterList"),
        Segment::Doc,
        Segment::Key("target"),
    ]
}

/// Path to the document of the element `index` of `target`.
pub(super) fn element(mut target: Path, index: usize) -> Path {
    target.push(Segment::Index(index));
    target.push(Segment::Doc);
    target
}

pub(super) fn with(mut path: Path, tail: &[Segment<'static>]) -> Path {
    path.extend_from_slice(tail);
    path
}
