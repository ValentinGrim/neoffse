//! `growth.toml`: base HP per class and total experience per level.
//!
//! The rules come from `docs/formats/pixel-remaster.md#maximum-hp-and-experience-tables`.
//! The raw structs refuse unknown fields to guard the schema of
//! `tools/pr-tables`.

use serde::Deserialize;

use crate::error::DataError;
use crate::parse::{read, rise};

const NAME: &str = "growth.toml";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GrowthFile {
    character_status: Vec<StatusRow>,
    exp_table: Vec<ExpRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusRow {
    id: u32,
    hp: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpRow {
    lv: u32,
    total_exp: u32,
}

/// The growth tables of one game.
///
/// `base_hp` holds `(characterStatusId, hp)` pairs ascending by id.
/// `exp` is indexed by `level - 1` and rises strictly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Growth {
    base_hp: Vec<(u32, u32)>,
    exp: Vec<u32>,
}

impl Growth {
    /// The base HP of the class with that `characterStatusId`.
    pub(crate) fn base_hp(&self, status_id: u32) -> Option<u32> {
        let index = self.base_hp.binary_search_by_key(&status_id, |row| row.0).ok()?;
        self.base_hp.get(index).map(|row| row.1)
    }

    /// The total experience to reach `level`; `None` for level 0 and above the last row.
    pub(crate) fn exp_to_reach(&self, level: u32) -> Option<u32> {
        let index = usize::try_from(level.checked_sub(1)?).ok()?;
        self.exp.get(index).copied()
    }
}

fn rule(row: String, rule: &'static str) -> DataError {
    DataError::Rule { file: NAME, row, rule }
}

/// Parses the text of `growth.toml` and checks its rules.
pub(crate) fn parse_growth(text: &str) -> Result<Growth, DataError> {
    let file: GrowthFile = read(NAME, text)?;
    if file.character_status.is_empty() {
        return Err(rule("character_status".to_string(), "table has no row"));
    }
    if file.exp_table.is_empty() {
        return Err(rule("exp_table".to_string(), "table has no row"));
    }
    let mut last = 0;
    let mut base_hp = Vec::with_capacity(file.character_status.len());
    for row in &file.character_status {
        rise(NAME, &mut last, row.id)?;
        base_hp.push((row.id, row.hp));
    }
    // Levels index `exp` by position: a gap would serve the wrong row.
    let mut last: u32 = 0;
    let mut previous: Option<u32> = None;
    let mut exp = Vec::with_capacity(file.exp_table.len());
    for row in &file.exp_table {
        let expected = last.saturating_add(1);
        rise(NAME, &mut last, row.lv)?;
        if row.lv != expected {
            return Err(rule(format!("lv {}", row.lv), "level does not follow the level before it"));
        }
        if previous.is_some_and(|before| row.total_exp <= before) {
            return Err(rule(format!("lv {}", row.lv), "total_exp does not rise"));
        }
        previous = Some(row.total_exp);
        exp.push(row.total_exp);
    }
    Ok(Growth { base_hp, exp })
}
