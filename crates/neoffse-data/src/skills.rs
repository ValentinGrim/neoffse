//! `skills.toml`: the base of each character status, the proficiency curve and
//! the spell chains of FF II.
//!
//! The rules come from `docs/formats/pixel-remaster.md#character-growth-in-ff-ii`,
//! `#skill-levels-in-ff-ii` and `#spells-in-ff-ii`. The raw structs refuse
//! unknown fields to guard the schema of `tools/pr-tables`.

use std::collections::BTreeMap;

use neoffse_core::TableKind;
use serde::Deserialize;

use crate::entry::Entry;
use crate::error::DataError;
use crate::parse::{read, rise};

const NAME: &str = "skills.toml";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillsFile {
    status: Vec<StatusRow>,
    skill_curve: SkillCurve,
    spell_chains: SpellChains,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusRow {
    id: u32,
    hp: u32,
    mp: u32,
    power: u32,
    vitality: u32,
    agility: u32,
    intelligence: u32,
    spirit: u32,
    magic: u32,
}

/// The `[skill_curve]` table: index 0 is skill level 1.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SkillCurve {
    total_exp: Vec<u32>,
}

/// The `[spell_chains]` table: the first ability id of each chain and the
/// number of rows in every chain.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpellChains {
    first: Vec<u32>,
    length: u32,
}

/// The base of one `character_status.csv` row
/// (`docs/formats/pixel-remaster.md#character-growth-in-ff-ii`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct BaseStats {
    /// `hp`.
    pub hp: u32,
    /// `mp`.
    pub mp: u32,
    /// The column `strength`.
    pub power: u32,
    /// `vitality`.
    pub vitality: u32,
    /// `agility`.
    pub agility: u32,
    /// `intelligence`.
    pub intelligence: u32,
    /// `spirit`.
    pub spirit: u32,
    /// `magic`.
    pub magic: u32,
}

/// The place of an ability in the spell chains
/// (`docs/formats/pixel-remaster.md#spells-in-ff-ii`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct SpellStep {
    /// The chain, counted from 0 in id order.
    pub chain: u32,
    /// The spell level in the chain, 1 to the chain length.
    pub level: u32,
}

/// The skill tables of one game.
///
/// `base` is ascending by status id. `total` is indexed by `level - 1` and
/// rises strictly. `first` holds the first ability id of each chain, and every
/// chain holds `length` consecutive ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Skills {
    base: Vec<(u32, BaseStats)>,
    total: Vec<u32>,
    first: Vec<u32>,
    length: u32,
}

// Each lookup answers `None` for an id the tables lack, as the accessors document.
// The `try_from` conversions between `usize` and `u32` cannot fail below the size
// of the chain list, so their `.ok()?` drops no error.
impl Skills {
    pub(crate) fn base_stats(&self, status_id: u32) -> Option<BaseStats> {
        let index = self.base.binary_search_by_key(&status_id, |row| row.0).ok()?;
        self.base.get(index).map(|row| row.1)
    }

    /// The total proficiency of `level`; `None` for level 0 and above the last row.
    pub(crate) fn skill_total(&self, level: u32) -> Option<u32> {
        let index = usize::try_from(level.checked_sub(1)?).ok()?;
        self.total.get(index).copied()
    }

    /// The chain that holds `ability_id`: the last chain starting at or before
    /// it, when the id falls inside its `length` rows.
    pub(crate) fn spell_step(&self, ability_id: u32) -> Option<SpellStep> {
        let chain = self.first.partition_point(|first| *first <= ability_id).checked_sub(1)?;
        let offset = ability_id - self.first.get(chain)?;
        if offset >= self.length {
            return None;
        }
        Some(SpellStep {
            chain: u32::try_from(chain).ok()?,
            level: offset + 1,
        })
    }

    pub(crate) fn spell_ability(&self, chain: u32, level: u32) -> Option<u32> {
        if level == 0 || level > self.length {
            return None;
        }
        let first = self.first.get(usize::try_from(chain).ok()?)?;
        first.checked_add(level - 1)
    }

    pub(crate) fn chain_count(&self) -> u32 {
        // Every chain starts at its own ability row, so the count stays far
        // below `u32::MAX`; the saturation only types the conversion.
        u32::try_from(self.first.len()).unwrap_or(u32::MAX)
    }
}

fn rule(row: String, rule: &'static str) -> DataError {
    DataError::Rule { file: NAME, row, rule }
}

/// Parses the text of `skills.toml` and checks its rules. `abilities` is the
/// parsed ability table: a spell level writes the content id of the ability.
pub(crate) fn parse_skills(text: &str, bounds: &BTreeMap<String, i64>, abilities: &[Entry]) -> Result<Skills, DataError> {
    let file: SkillsFile = read(NAME, text)?;
    if file.status.is_empty() {
        return Err(rule("status".to_string(), "table has no row"));
    }
    if let Some(first) = file.status.first().filter(|row| row.id != 1) {
        return Err(rule(format!("status id {}", first.id), "first status id is not 1"));
    }
    let mut last = 0;
    let mut base = Vec::with_capacity(file.status.len());
    for row in &file.status {
        rise(NAME, &mut last, row.id)?;
        base.push((
            row.id,
            BaseStats {
                hp: row.hp,
                mp: row.mp,
                power: row.power,
                vitality: row.vitality,
                agility: row.agility,
                intelligence: row.intelligence,
                spirit: row.spirit,
                magic: row.magic,
            },
        ));
    }
    let total = file.skill_curve.total_exp;
    check_curve(&total, bounds)?;
    let SpellChains { first, length } = file.spell_chains;
    check_chains(&first, length, abilities)?;
    Ok(Skills { base, total, first, length })
}

/// Level 1 holds 0 and the totals rise strictly. The row count equals the
/// level caps of `bounds.toml` when it names them
/// (`docs/formats/pixel-remaster.md#skill-levels-in-ff-ii`).
fn check_curve(total: &[u32], bounds: &BTreeMap<String, i64>) -> Result<(), DataError> {
    if total.first() != Some(&0) {
        return Err(rule("skill_curve lv 1".to_string(), "total_exp of level 1 is not 0"));
    }
    for (index, pair) in total.windows(2).enumerate() {
        if pair.get(1) <= pair.first() {
            // `windows(2)` starts at level 1, so the second value is level index + 2.
            return Err(rule(format!("skill_curve lv {}", index + 2), "total_exp does not rise"));
        }
    }
    for key in ["MAX_MAGICAL_LEVEL", "MAX_WEAPON_LEVEL"] {
        if let Some(cap) = bounds.get(key)
            && i64::try_from(total.len()) != Ok(*cap)
        {
            return Err(rule("skill_curve".to_string(), "row count differs from the level caps"));
        }
    }
    Ok(())
}

/// The chains start at least `length` apart, rise, and every id of every chain
/// is an ability row with a content id.
fn check_chains(first: &[u32], length: u32, abilities: &[Entry]) -> Result<(), DataError> {
    if first.is_empty() {
        return Err(rule("spell_chains".to_string(), "table has no row"));
    }
    if length == 0 {
        return Err(rule("spell_chains".to_string(), "length is below 1"));
    }
    for (index, pair) in first.windows(2).enumerate() {
        let (Some(before), Some(after)) = (pair.first(), pair.get(1)) else {
            continue;
        };
        if before.checked_add(length).is_none_or(|end| *after < end) {
            // `windows(2)` starts at chain 0, so the second value is chain index + 1.
            return Err(rule(format!("spell_chains chain {}", index + 1), "chain starts inside the chain before it"));
        }
    }
    for (chain, start) in first.iter().enumerate() {
        for id in *start..start.saturating_add(length) {
            let row = abilities.binary_search_by_key(&id, |entry| entry.id).ok().and_then(|at| abilities.get(at));
            if row.and_then(|entry| entry.ability).and_then(|info| info.content_id).is_none() {
                return Err(rule(format!("spell_chains chain {chain} id {id}"), "id is no ability with a content id"));
            }
        }
    }
    Ok(())
}

/// The ability table of a parsed set, for the cross-file check.
pub(crate) fn ability_rows(tables: &BTreeMap<TableKind, Vec<Entry>>) -> &[Entry] {
    tables.get(&TableKind::Ability).map_or(&[], Vec::as_slice)
}
