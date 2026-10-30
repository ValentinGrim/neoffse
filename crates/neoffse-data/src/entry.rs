//! The rows of a table.

use neoffse_core::TableKind;

/// A language the tables carry names in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Language {
    /// English.
    En,
    /// French.
    Fr,
}

impl Language {
    /// Every language, in declaration order.
    pub const ALL: &[Language] = &[Language::En, Language::Fr];
}

/// The kind of an ability row, one reading of its `type_id` for the six
/// games (`docs/formats/pixel-remaster.md#ability-types`). The game names no
/// kind: the tool derives it from the row. A kind is a label of the tool, so
/// read [`AbilityInfo::type_id`] where the exact game value matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AbilityKind {
    /// A white magic spell.
    WhiteMagic,
    /// A black magic spell.
    BlackMagic,
    /// `type_id` 3 and 13. FF I names them monster abilities; the label misleads
    /// in FF V and FF VI (`docs/formats/pixel-remaster.md#ability-types`).
    Monster,
    /// A command ability.
    Command,
    /// A summon spell.
    Summon,
    /// A ninjutsu spell.
    Ninjutsu,
    /// An ability of the game's `Other` type (`type_id` 11).
    Other,
    /// A time magic spell.
    TimeMagic,
    /// A magic sword spell.
    MagicSword,
    /// A blue magic spell.
    BlueMagic,
    /// A hissatsu technique.
    Hissatsu,
    /// A special ability.
    SpecialAbility,
    /// A dance.
    Dance,
    /// A madou spell.
    Madou,
    /// A rampage.
    Rampage,
    /// An ability the game runs by itself, never from a menu.
    Internal,
}

/// What an ability row adds to an [`Entry`].
///
/// `content_id` is the id the ability has inside an inventory or a spell
/// list, which is not the table id (`docs/formats/pixel-remaster.md#abilities`).
/// It is no id of the content tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AbilityInfo {
    /// The ability's content id; `None` for an ability the game has no content
    /// row for (`docs/formats/pixel-remaster.md#rows-without-a-name`). Never 0.
    pub content_id: Option<u32>,
    /// `ability.csv` `type_id` as the game stores it: a value of the game's
    /// `AbilityType` enum (`docs/formats/pixel-remaster.md#ability-types`).
    pub type_id: u32,
    /// The ability's kind.
    pub kind: AbilityKind,
    /// The spell level, 0 to 8; 0 for rows that are no spell.
    pub level: u8,
}

/// One row of a table, in both languages.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Entry {
    /// The id, at least 1, unique inside its table.
    pub id: u32,
    /// The table the entry belongs to. A placeholder keeps the kind of its
    /// equipment table (`docs/formats/pixel-remaster.md#equipment-slots`).
    pub kind: TableKind,
    /// True for the empty-slot row of an equipment table.
    pub placeholder: bool,
    /// The English name, as the file spells it. `None` for a row the game names
    /// with nothing (`docs/formats/pixel-remaster.md#rows-without-a-name`). Set
    /// exactly when `name_fr` is.
    pub name_en: Option<String>,
    /// The French name, as the file spells it. `None` exactly when `name_en` is.
    pub name_fr: Option<String>,
    /// Some exactly for the rows of [`TableKind::Ability`].
    pub ability: Option<AbilityInfo>,
}

impl Entry {
    /// The name in `language`; `None` for a row without a name. An empty name
    /// is a name: it comes back as `Some("")`.
    #[must_use]
    pub fn name(&self, language: Language) -> Option<&str> {
        match language {
            Language::En => self.name_en.as_deref(),
            Language::Fr => self.name_fr.as_deref(),
        }
    }
}
