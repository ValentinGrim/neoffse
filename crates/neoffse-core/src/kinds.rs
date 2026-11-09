//! Capabilities, ID table kinds and the cap policy.
//!
//! A [`CapPolicy`] says whether an edit respects the game's own cap. An
//! inverse edit, a rollback and a redo restore values the document already
//! held, and the user may have set those above the cap in expert mode, so the
//! facade applies them with [`CapPolicy::StorageOnly`]. A new edit gets
//! [`CapPolicy::Enforce`] unless the user turned expert mode on.

/// A section of the editor a save supports.
///
/// The editor shows one section per capability
/// (`docs/architecture.md#ui-architecture`, "Editor"). The Pixel Remaster
/// mapping reports `Party` and `Inventory`; `Magic` and `Bestiary` exist for
/// the sections the architecture names, and no format reports them yet. Gil
/// sits in `Inventory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Capability {
    /// The characters of the party and their parameters.
    Party,
    /// Items, key items, equipment stock and gil.
    Inventory,
    /// Spells and abilities a character knows.
    Magic,
    /// Monsters met.
    Bestiary,
}

/// An ID table a field can reference. The first twelve variants are the
/// tables of `docs/formats/pixel-remaster.md#go-editor-tables`; `Keyword`,
/// `MagicStone` and `CommandContent` hold the content types 6, 9 and 7
/// (`docs/formats/pixel-remaster.md#content-types-5-to-9`).
///
/// `neoffse-data` keys its tables by this type. Finer kinds inside a table (a
/// spell against a monster ability) are entry kinds, not table kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum TableKind {
    /// The `Items` table.
    Item,
    /// The `ImportantItems` table.
    KeyItem,
    /// The `Weapons` table.
    Weapon,
    /// The `Shields` table.
    Shield,
    /// The `Armors` table.
    Armor,
    /// The `Helmets` table.
    Helmet,
    /// The `Gloves` table.
    Gloves,
    /// The `Abilities` table.
    Ability,
    /// The `Jobs` table.
    Job,
    /// The `Commands` table.
    Command,
    /// The `Maps` table.
    Map,
    /// The `Bestiary` table.
    Monster,
    /// The learned words of FF II (content type 6).
    Keyword,
    /// The espers of FF VI (content type 9).
    MagicStone,
    /// The content rows of type 7 (FF V and FF VI), keyed by content ID and
    /// not by `command.csv` ID, so apart from [`TableKind::Command`].
    CommandContent,
}

/// Whether an edit must respect the game's own caps.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CapPolicy {
    /// Storage bounds, the game cap and the ID tables. A value above the cap
    /// of the field's metadata is refused with `EditError::AboveCap`, and an
    /// inserted key outside its ID table with `EditError::UnknownId`. The
    /// default.
    #[default]
    Enforce,
    /// Storage bounds only: accepts anything the save can hold. This is the
    /// "expert mode" of the app, and the policy of every inverse edit,
    /// rollback and redo.
    StorageOnly,
}
