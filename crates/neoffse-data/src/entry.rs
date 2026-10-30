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

/// The school of an ability row
/// (`docs/formats/pixel-remaster.md#ability-job-and-command-tables`). The
/// game names no school: the tool derives it from the row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AbilityKind {
    /// A white magic spell.
    WhiteMagic,
    /// A black magic spell.
    BlackMagic,
    /// An ability only monsters use.
    Monster,
    /// A command ability.
    Command,
}

/// What an ability row adds to an [`Entry`].
///
/// `content_id` is the id the ability has inside an inventory or a spell
/// list, which is not the table id (`docs/formats/pixel-remaster.md#abilities`).
/// It is no id of the content tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AbilityInfo {
    /// The ability's content id.
    pub content_id: u32,
    /// The ability's school.
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
