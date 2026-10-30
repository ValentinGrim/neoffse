//! `growth.toml`: base stats per class, total experience per level and the
//! growth rows of the level-up rule.
//!
//! The rules come from `docs/formats/pixel-remaster.md#maximum-hp-and-experience-tables`
//! and `docs/formats/pixel-remaster.md#level-up-in-ff-i`. The raw structs
//! refuse unknown fields to guard the schema of `tools/pr-tables`.

use serde::Deserialize;

use crate::curve::{Curve, CurveGroup, JobRow, parse_curve};
use crate::error::DataError;
use crate::parse::{read, rise};

const NAME: &str = "growth.toml";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrowthFile {
    character_status: Vec<StatusRow>,
    exp_table: ExpTable,
    intermediate_growth_curve: Vec<JobRow>,
    growth_curve: Vec<CurveGroup>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusRow {
    id: u32,
    hp: u32,
    vitality: u32,
}

/// The `[exp_table]` table: index 0 is level 1.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpTable {
    total_exp: Vec<u32>,
}

/// One row of `growth_curve.csv`: what reaching one level adds
/// (`docs/formats/pixel-remaster.md#level-up-in-ff-i`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct GrowthStep {
    /// `hp_value1` is 1: the strong HP bonus is forced.
    pub strong_hp: bool,
    /// strength, vitality, agility, intelligence, spirit, magic, luck, in the
    /// rule's order. A non-zero value is added; a 0 rolls 1 in 8.
    pub stats: [u32; 7],
    /// `accuracy_rate`: added with no roll.
    pub accuracy_rate: u32,
    /// `magic_evasion_rate`: added with no roll.
    pub magic_evasion_rate: u32,
    /// `magical_times1` to `magical_times8`: the spell charges of the row.
    pub charges: [u32; 8],
}

/// The growth tables of one game.
///
/// `base` holds `(characterStatusId, hp, vitality)` ascending by id. `exp` is
/// indexed by `level - 1` and rises strictly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Growth {
    base: Vec<(u32, u32, u32)>,
    exp: Vec<u32>,
    curve: Curve,
}

impl Growth {
    fn status(&self, status_id: u32) -> Option<&(u32, u32, u32)> {
        let index = self.base.binary_search_by_key(&status_id, |row| row.0).ok()?;
        self.base.get(index)
    }

    /// The base HP of the class with that `characterStatusId`.
    pub(crate) fn base_hp(&self, status_id: u32) -> Option<u32> {
        self.status(status_id).map(|row| row.1)
    }

    /// The base vitality of the class with that `characterStatusId`.
    pub(crate) fn base_vitality(&self, status_id: u32) -> Option<u32> {
        self.status(status_id).map(|row| row.2)
    }

    /// The total experience to reach `level`; `None` for level 0 and above the last row.
    pub(crate) fn exp_to_reach(&self, level: u32) -> Option<u32> {
        let index = usize::try_from(level.checked_sub(1)?).ok()?;
        self.exp.get(index).copied()
    }

    /// The growth row of the job's group at `level`.
    pub(crate) fn growth_step(&self, job_id: u32, level: u32) -> Option<GrowthStep> {
        self.curve.step(job_id, level)
    }
}

pub(crate) fn rule(row: String, rule: &'static str) -> DataError {
    DataError::Rule { file: NAME, row, rule }
}

/// Parses the text of `growth.toml` and checks its rules.
pub(crate) fn parse_growth(text: &str) -> Result<Growth, DataError> {
    let file: GrowthFile = read(NAME, text)?;
    if file.character_status.is_empty() {
        return Err(rule("character_status".to_string(), "table has no row"));
    }
    if file.exp_table.total_exp.is_empty() {
        return Err(rule("exp_table".to_string(), "table has no row"));
    }
    let mut last = 0;
    let mut base = Vec::with_capacity(file.character_status.len());
    for row in &file.character_status {
        rise(NAME, &mut last, row.id)?;
        base.push((row.id, row.hp, row.vitality));
    }
    let exp = file.exp_table.total_exp;
    for (index, pair) in exp.windows(2).enumerate() {
        if pair.get(1) <= pair.first() {
            // `windows(2)` starts at level 1, so the second value is level index + 2.
            return Err(rule(format!("exp_table lv {}", index + 2), "total_exp does not rise"));
        }
    }
    let curve = parse_curve(&file.intermediate_growth_curve, &file.growth_curve, exp.len())?;
    Ok(Growth { base, exp, curve })
}
