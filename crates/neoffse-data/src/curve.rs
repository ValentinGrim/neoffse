//! The growth rows of `growth.toml`: jobs to growth groups, and one column
//! array per group.
//!
//! The layout is in `docs/formats/pixel-remaster.md#maximum-hp-and-experience-tables`
//! ("Growth tables") and the rule that reads it in
//! `docs/formats/pixel-remaster.md#level-up-in-ff-i`. A column array is
//! indexed by `level - first_lv`.

use std::array;

use serde::Deserialize;

use crate::error::DataError;
use crate::growth::{GrowthStep, rule};
use crate::parse::rise;

const NAME: &str = "growth.toml";

/// The level of index 0 in every group: `growth_curve.csv` has no level 1 row.
const FIRST_LEVEL: usize = 2;

/// One `[[intermediate_growth_curve]]` entry.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JobRow {
    job_id: u32,
    growth_curve_group_id: u32,
}

/// One `[[growth_curve]]` entry: a growth group, one array per column.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CurveGroup {
    group_id: u32,
    first_lv: u32,
    hp_value1: Vec<u32>,
    strength: Vec<u32>,
    vitality: Vec<u32>,
    agility: Vec<u32>,
    intelligence: Vec<u32>,
    spirit: Vec<u32>,
    magic: Vec<u32>,
    luck: Vec<u32>,
    accuracy_rate: Vec<u32>,
    magic_evasion_rate: Vec<u32>,
    magical_times1: Vec<u32>,
    magical_times2: Vec<u32>,
    magical_times3: Vec<u32>,
    magical_times4: Vec<u32>,
    magical_times5: Vec<u32>,
    magical_times6: Vec<u32>,
    magical_times7: Vec<u32>,
    magical_times8: Vec<u32>,
}

impl CurveGroup {
    /// Every column with the rule text for a wrong length, in file order.
    /// `DataError::Rule` takes a `&'static str`, so each column has a literal.
    fn columns(&self) -> [(&'static str, &[u32]); 18] {
        [
            ("hp_value1 does not cover the levels of exp_table", &self.hp_value1),
            ("strength does not cover the levels of exp_table", &self.strength),
            ("vitality does not cover the levels of exp_table", &self.vitality),
            ("agility does not cover the levels of exp_table", &self.agility),
            ("intelligence does not cover the levels of exp_table", &self.intelligence),
            ("spirit does not cover the levels of exp_table", &self.spirit),
            ("magic does not cover the levels of exp_table", &self.magic),
            ("luck does not cover the levels of exp_table", &self.luck),
            ("accuracy_rate does not cover the levels of exp_table", &self.accuracy_rate),
            ("magic_evasion_rate does not cover the levels of exp_table", &self.magic_evasion_rate),
            ("magical_times1 does not cover the levels of exp_table", &self.magical_times1),
            ("magical_times2 does not cover the levels of exp_table", &self.magical_times2),
            ("magical_times3 does not cover the levels of exp_table", &self.magical_times3),
            ("magical_times4 does not cover the levels of exp_table", &self.magical_times4),
            ("magical_times5 does not cover the levels of exp_table", &self.magical_times5),
            ("magical_times6 does not cover the levels of exp_table", &self.magical_times6),
            ("magical_times7 does not cover the levels of exp_table", &self.magical_times7),
            ("magical_times8 does not cover the levels of exp_table", &self.magical_times8),
        ]
    }

    /// The rows of the group, level `FIRST_LEVEL` first. Call it after the
    /// length check: a short column would read as 0.
    fn steps(&self) -> Vec<GrowthStep> {
        let columns = self.columns();
        (0..self.hp_value1.len())
            .map(|index| {
                let at = |column: usize| columns.get(column).and_then(|(_, values)| values.get(index)).copied().unwrap_or_default();
                GrowthStep {
                    strong_hp: at(0) == 1,
                    stats: array::from_fn(|stat| at(1 + stat)),
                    accuracy_rate: at(8),
                    magic_evasion_rate: at(9),
                    charges: array::from_fn(|spell| at(10 + spell)),
                }
            })
            .collect()
    }
}

/// The checked growth rows.
///
/// `jobs` holds `(job_id, growth_curve_group_id)` ascending by job, `groups`
/// holds `(group_id, rows)` ascending by group, rows indexed by
/// `level - FIRST_LEVEL`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Curve {
    jobs: Vec<(u32, u32)>,
    groups: Vec<(u32, Vec<GrowthStep>)>,
}

impl Curve {
    /// The row of the job's growth group at `level`.
    pub(crate) fn step(&self, job_id: u32, level: u32) -> Option<GrowthStep> {
        let job = self.jobs.binary_search_by_key(&job_id, |row| row.0).ok()?;
        let group_id = self.jobs.get(job)?.1;
        let group = self.groups.binary_search_by_key(&group_id, |row| row.0).ok()?;
        let index = usize::try_from(level).ok()?.checked_sub(FIRST_LEVEL)?;
        self.groups.get(group)?.1.get(index).copied()
    }
}

/// Checks the rows against `last_level`, the last level of `exp_table`.
pub(crate) fn parse_curve(jobs: &[JobRow], groups: &[CurveGroup], last_level: usize) -> Result<Curve, DataError> {
    if jobs.is_empty() {
        return Err(rule("intermediate_growth_curve".to_string(), "table has no row"));
    }
    if groups.is_empty() {
        return Err(rule("growth_curve".to_string(), "table has no row"));
    }
    let mut last = 0;
    let mut kept = Vec::with_capacity(groups.len());
    for group in groups {
        let row = format!("growth_curve group {}", group.group_id);
        if group.group_id <= last {
            return Err(rule(row, "group does not rise"));
        }
        last = group.group_id;
        if usize::try_from(group.first_lv) != Ok(FIRST_LEVEL) {
            return Err(rule(row, "group does not start at level 2"));
        }
        let expected = (last_level + 1).saturating_sub(FIRST_LEVEL);
        for (message, values) in group.columns() {
            if values.len() != expected {
                return Err(rule(row, message));
            }
        }
        if group.hp_value1.iter().any(|flag| *flag > 1) {
            return Err(rule(row, "hp_value1 is neither 0 nor 1"));
        }
        kept.push((group.group_id, group.steps()));
    }
    let mut last = 0;
    let mut pairs = Vec::with_capacity(jobs.len());
    for job in jobs {
        rise(NAME, &mut last, job.job_id)?;
        if kept.binary_search_by_key(&job.growth_curve_group_id, |row| row.0).is_err() {
            let row = format!("intermediate_growth_curve job {}", job.job_id);
            return Err(rule(row, "growth group has no growth_curve row"));
        }
        pairs.push((job.job_id, job.growth_curve_group_id));
    }
    Ok(Curve { jobs: pairs, groups: kept })
}
