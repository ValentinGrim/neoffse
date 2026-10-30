//! The parsed tables of one game.

use std::collections::BTreeMap;

use neoffse_core::{Game, TableKind};

use crate::entry::Entry;
use crate::error::DataError;
use crate::growth::{Growth, GrowthStep};
use crate::parse::parse_sources;

/// The seven tables whose ids share one content id space
/// (`docs/formats/pixel-remaster.md#id-spaces-and-tables`).
const CONTENT_KINDS: [TableKind; 7] = [
    TableKind::Item,
    TableKind::KeyItem,
    TableKind::Weapon,
    TableKind::Shield,
    TableKind::Armor,
    TableKind::Helmet,
    TableKind::Gloves,
];

/// The text of the seven files of one game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sources<'a> {
    /// `content.toml`: items, key items and equipment.
    pub content: &'a str,
    /// `abilities.toml`.
    pub abilities: &'a str,
    /// `jobs.toml`.
    pub jobs: &'a str,
    /// `commands.toml`.
    pub commands: &'a str,
    /// `monsters.toml`: the bestiary.
    pub monsters: &'a str,
    /// `bounds.toml`: the game's caps.
    pub bounds: &'a str,
    /// `growth.toml`: base stats per class, experience per level and growth rows.
    pub growth: &'a str,
}

/// The immutable tables and caps of one game.
///
/// Lookups use ids. A name is never a key: names repeat inside a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameTables {
    game: Game,
    tables: BTreeMap<TableKind, Vec<Entry>>,
    bounds: BTreeMap<String, i64>,
    growth: Growth,
}

impl GameTables {
    /// Parses the seven texts of `game` in the order content, abilities, jobs,
    /// commands, monsters, bounds, growth. The function reads no disk and keeps no
    /// global state.
    ///
    /// # Errors
    ///
    /// Returns the first [`DataError`]: a TOML error, a `kind` string it does
    /// not know, an id that is below 1 or does not rise above the id before
    /// it in its file, or a row that breaks a rule of the file contract
    /// ([`DataError::Rule`]): an ability level above 8, an ability
    /// `content_id` of 0, a placeholder written as `false`, a placeholder
    /// outside the five equipment kinds or repeated in one kind, a cap key
    /// without the `MAX_` prefix, or a negative cap. `growth.toml` adds its
    /// own rules: status ids rise from 1, `total_exp` starts at 0 and rises
    /// strictly, `exp_table` has `MAX_LV` rows when `bounds.toml` holds that
    /// cap, job and group ids rise, every group starts at level 2 and holds one value per
    /// level of `exp_table` in each column, `hp_value1` is 0 or 1, every job
    /// names a group that exists, and no table is empty.
    pub fn parse(game: Game, sources: &Sources<'_>) -> Result<GameTables, DataError> {
        let parsed = parse_sources(sources)?;
        Ok(GameTables {
            game,
            tables: parsed.tables,
            bounds: parsed.bounds,
            growth: parsed.growth,
        })
    }

    /// The game these tables describe.
    #[must_use]
    pub fn game(&self) -> Game {
        self.game
    }

    /// The entries of one table, ascending by id, placeholders included.
    /// `None` when the game has no table of that kind (`TableKind::Map`
    /// today). A present table is never empty.
    #[must_use]
    pub fn table(&self, kind: TableKind) -> Option<&[Entry]> {
        self.tables.get(&kind).map(Vec::as_slice)
    }

    /// The entry of `kind` with that id. An id of another table gives `None`.
    #[must_use]
    pub fn entry(&self, kind: TableKind, id: u32) -> Option<&Entry> {
        let rows = self.table(kind)?;
        rows.binary_search_by_key(&id, |entry| entry.id).ok().map(|index| &rows[index])
    }

    /// The entry with that content id in any of the seven content tables
    /// (items, key items and the five equipment kinds). Ability content ids
    /// are not in this space: see [`GameTables::ability_by_content`].
    #[must_use]
    pub fn content(&self, id: u32) -> Option<&Entry> {
        CONTENT_KINDS.iter().find_map(|kind| self.entry(*kind, id))
    }

    /// The first entry of `table(kind)` whose `placeholder` is true.
    #[must_use]
    pub fn placeholder(&self, kind: TableKind) -> Option<&Entry> {
        self.table(kind)?.iter().find(|entry| entry.placeholder)
    }

    /// The first ability, in id order, whose `content_id` matches
    /// (`docs/formats/pixel-remaster.md#abilities`).
    #[must_use]
    pub fn ability_by_content(&self, content_id: u32) -> Option<&Entry> {
        self.table(TableKind::Ability)?
            .iter()
            .find(|entry| entry.ability.is_some_and(|info| info.content_id == content_id))
    }

    /// The game's cap under its own spelling (`"MAX_GIL"`, `"MAX_STRNGTH"`).
    /// A cap is a game rule, never a storage bound
    /// (`docs/formats/pixel-remaster.md#bounds-in-the-master-data`).
    #[must_use]
    pub fn bound(&self, key: &str) -> Option<i64> {
        self.bounds.get(key).copied()
    }

    /// Every cap, ascending by key.
    pub fn bounds(&self) -> impl Iterator<Item = (&str, i64)> {
        self.bounds.iter().map(|(key, value)| (key.as_str(), *value))
    }

    /// The base HP of the `character_status` row with that id (the
    /// `characterStatusId` of a save). `None` for an id without a row.
    #[must_use]
    pub fn base_hp(&self, status_id: u32) -> Option<u32> {
        self.growth.base_hp(status_id)
    }

    /// The total experience a character needs to reach `level`: 0 at level 1.
    /// `None` for level 0 and for a level above the last row (99 in FF I).
    #[must_use]
    pub fn exp_to_reach(&self, level: u32) -> Option<u32> {
        self.growth.exp_to_reach(level)
    }

    /// The base vitality of the `character_status` row with that id (the
    /// `characterStatusId` of a save), the base of the HP rule
    /// (`docs/formats/pixel-remaster.md#level-up-in-ff-i`). `None` for an id
    /// without a row.
    #[must_use]
    pub fn base_vitality(&self, status_id: u32) -> Option<u32> {
        self.growth.base_vitality(status_id)
    }

    /// The growth row of the job's growth group at `level`. `None` for a job
    /// without a row in `intermediate_growth_curve`, for level 0 and 1, and
    /// above the last level of the group (99 in FF I).
    #[must_use]
    pub fn growth_step(&self, job_id: u32, level: u32) -> Option<GrowthStep> {
        self.growth.growth_step(job_id, level)
    }
}
