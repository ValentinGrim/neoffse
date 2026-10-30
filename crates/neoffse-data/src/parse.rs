//! The parse of a set of texts: file order, cap checks and the rules that span files.

use std::collections::BTreeMap;

use neoffse_core::TableKind;
use serde::Deserialize;

use crate::entry::Entry;
use crate::error::DataError;
use crate::growth::{Growth, parse_growth};
use crate::rows::{CommandFile, JobFile, MonsterFile, abilities, content, names};
use crate::tables::Sources;

/// The tables and caps of one game, before `GameTables` wraps them.
pub(crate) struct Parsed {
    pub(crate) tables: BTreeMap<TableKind, Vec<Entry>>,
    pub(crate) bounds: BTreeMap<String, i64>,
    pub(crate) growth: Option<Growth>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundsFile {
    bounds: BTreeMap<String, i64>,
}

/// Parses the texts in file order and returns the first error. `monsters` and
/// `growth` are read only when present.
pub(crate) fn parse_sources(sources: &Sources<'_>) -> Result<Parsed, DataError> {
    let mut tables = BTreeMap::new();
    content(&read("content.toml", sources.content)?, &mut tables)?;
    abilities(&read("abilities.toml", sources.abilities)?, &mut tables)?;
    let jobs: JobFile = read("jobs.toml", sources.jobs)?;
    names("jobs.toml", &jobs.job, TableKind::Job, &mut tables)?;
    let commands: CommandFile = read("commands.toml", sources.commands)?;
    names("commands.toml", &commands.command, TableKind::Command, &mut tables)?;
    if let Some(text) = sources.monsters {
        let monsters: MonsterFile = read("monsters.toml", text)?;
        names("monsters.toml", &monsters.monster, TableKind::Monster, &mut tables)?;
    }
    let bounds: BoundsFile = read("bounds.toml", sources.bounds)?;
    check_bounds(&bounds.bounds)?;
    let growth = sources.growth.map(parse_growth).transpose()?;
    if let Some(growth) = &growth {
        check_level_cap(&bounds.bounds, growth)?;
    }
    Ok(Parsed {
        tables,
        bounds: bounds.bounds,
        growth,
    })
}

pub(crate) fn rule(file: &'static str, row: String, rule: &'static str) -> DataError {
    DataError::Rule { file, row, rule }
}

/// Every key starts with `MAX_` and no cap is negative.
fn check_bounds(bounds: &BTreeMap<String, i64>) -> Result<(), DataError> {
    for (key, value) in bounds {
        if !key.starts_with("MAX_") {
            return Err(rule("bounds.toml", key.clone(), "key without the MAX_ prefix"));
        }
        if *value < 0 {
            return Err(rule("bounds.toml", key.clone(), "negative bound"));
        }
    }
    Ok(())
}

/// `exp_table` holds one row per level up to `MAX_LV`, when `bounds` names it
/// (`docs/formats/pixel-remaster.md#bounds-in-the-master-data`). Without that key
/// the check is skipped.
fn check_level_cap(bounds: &BTreeMap<String, i64>, growth: &Growth) -> Result<(), DataError> {
    match bounds.get("MAX_LV") {
        Some(cap) if i64::try_from(growth.level_count()) != Ok(*cap) => Err(rule("growth.toml", "exp_table".to_string(), "row count differs from MAX_LV")),
        _ => Ok(()),
    }
}

pub(crate) fn read<'de, T: Deserialize<'de>>(file: &'static str, text: &'de str) -> Result<T, DataError> {
    toml::from_str(text).map_err(|source| DataError::Toml {
        file,
        source: Box::new(source),
    })
}

/// Requires each id to be at least 1 and above the id before it.
pub(crate) fn rise(file: &'static str, last: &mut u32, id: u32) -> Result<(), DataError> {
    if id <= *last {
        return Err(DataError::IdOrder { file, id });
    }
    *last = id;
    Ok(())
}
