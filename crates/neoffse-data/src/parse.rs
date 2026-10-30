//! Raw rows of the table files and their conversion to entries.
//!
//! Every raw struct refuses unknown fields: that guards the schema of
//! `tools/pr-tables`, not the content. A column the tool adds fails the
//! tests until this module handles it.

use std::collections::BTreeMap;

use neoffse_core::TableKind;
use serde::Deserialize;

use crate::entry::{AbilityInfo, AbilityKind, Entry};
use crate::error::DataError;
use crate::growth::{Growth, parse_growth};
use crate::tables::Sources;

/// The tables and caps of one game, before `GameTables` wraps them.
pub(crate) struct Parsed {
    pub(crate) tables: BTreeMap<TableKind, Vec<Entry>>,
    pub(crate) bounds: BTreeMap<String, i64>,
    pub(crate) growth: Growth,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentFile {
    content: Vec<ContentRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentRow {
    id: u32,
    kind: String,
    placeholder: Option<bool>,
    name_en: String,
    name_fr: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AbilityFile {
    ability: Vec<AbilityRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AbilityRow {
    id: u32,
    content_id: u32,
    kind: String,
    level: u8,
    name_en: String,
    name_fr: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NameRow {
    id: u32,
    name_en: String,
    name_fr: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JobFile {
    job: Vec<NameRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandFile {
    command: Vec<NameRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MonsterFile {
    monster: Vec<NameRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundsFile {
    bounds: BTreeMap<String, i64>,
}

/// Parses the seven texts in file order and returns the first error.
pub(crate) fn parse_sources(sources: &Sources<'_>) -> Result<Parsed, DataError> {
    let mut tables = BTreeMap::new();
    content(&read("content.toml", sources.content)?, &mut tables)?;
    abilities(&read("abilities.toml", sources.abilities)?, &mut tables)?;
    let jobs: JobFile = read("jobs.toml", sources.jobs)?;
    names("jobs.toml", &jobs.job, TableKind::Job, &mut tables)?;
    let commands: CommandFile = read("commands.toml", sources.commands)?;
    names("commands.toml", &commands.command, TableKind::Command, &mut tables)?;
    let monsters: MonsterFile = read("monsters.toml", sources.monsters)?;
    names("monsters.toml", &monsters.monster, TableKind::Monster, &mut tables)?;
    let bounds: BoundsFile = read("bounds.toml", sources.bounds)?;
    check_bounds(&bounds.bounds)?;
    let growth = parse_growth(sources.growth)?;
    check_level_cap(&bounds.bounds, &growth)?;
    Ok(Parsed {
        tables,
        bounds: bounds.bounds,
        growth,
    })
}

fn rule(file: &'static str, row: String, rule: &'static str) -> DataError {
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

/// The empty-slot flag of a content row: absent or `true`, only on an
/// equipment kind, once per kind.
fn check_placeholder(row: &ContentRow, kind: TableKind, seen: &mut Vec<TableKind>) -> Result<bool, DataError> {
    const NAME: &str = "content.toml";
    let Some(flag) = row.placeholder else {
        return Ok(false);
    };
    let at = || format!("id {}", row.id);
    if !flag {
        return Err(rule(NAME, at(), "placeholder written as false"));
    }
    if !matches!(
        kind,
        TableKind::Weapon | TableKind::Shield | TableKind::Armor | TableKind::Helmet | TableKind::Gloves
    ) {
        return Err(rule(NAME, at(), "placeholder on a non-equipment kind"));
    }
    if seen.contains(&kind) {
        return Err(rule(NAME, at(), "second placeholder of a kind"));
    }
    seen.push(kind);
    Ok(true)
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

fn content(file: &ContentFile, tables: &mut BTreeMap<TableKind, Vec<Entry>>) -> Result<(), DataError> {
    const NAME: &str = "content.toml";
    let mut last = 0;
    let mut seen = Vec::new();
    for row in &file.content {
        rise(NAME, &mut last, row.id)?;
        let kind = match row.kind.as_str() {
            "item" => TableKind::Item,
            "key_item" => TableKind::KeyItem,
            "weapon" => TableKind::Weapon,
            "shield" => TableKind::Shield,
            "armor" => TableKind::Armor,
            "helmet" => TableKind::Helmet,
            "gloves" => TableKind::Gloves,
            _ => {
                return Err(DataError::UnknownKind {
                    file: NAME,
                    id: row.id,
                    kind: row.kind.clone(),
                });
            }
        };
        let placeholder = check_placeholder(row, kind, &mut seen)?;
        tables.entry(kind).or_default().push(Entry {
            id: row.id,
            kind,
            placeholder,
            name_en: row.name_en.clone(),
            name_fr: row.name_fr.clone(),
            ability: None,
        });
    }
    Ok(())
}

fn abilities(file: &AbilityFile, tables: &mut BTreeMap<TableKind, Vec<Entry>>) -> Result<(), DataError> {
    const NAME: &str = "abilities.toml";
    let mut last = 0;
    for row in &file.ability {
        rise(NAME, &mut last, row.id)?;
        let kind = match row.kind.as_str() {
            "white_magic" => AbilityKind::WhiteMagic,
            "black_magic" => AbilityKind::BlackMagic,
            "monster" => AbilityKind::Monster,
            "command" => AbilityKind::Command,
            _ => {
                return Err(DataError::UnknownKind {
                    file: NAME,
                    id: row.id,
                    kind: row.kind.clone(),
                });
            }
        };
        if row.level > 8 {
            return Err(rule(NAME, format!("id {}", row.id), "level above 8"));
        }
        if row.content_id == 0 {
            return Err(rule(NAME, format!("id {}", row.id), "content_id 0"));
        }
        tables.entry(TableKind::Ability).or_default().push(Entry {
            id: row.id,
            kind: TableKind::Ability,
            placeholder: false,
            name_en: row.name_en.clone(),
            name_fr: row.name_fr.clone(),
            ability: Some(AbilityInfo {
                content_id: row.content_id,
                kind,
                level: row.level,
            }),
        });
    }
    Ok(())
}

fn names(file: &'static str, rows: &[NameRow], kind: TableKind, tables: &mut BTreeMap<TableKind, Vec<Entry>>) -> Result<(), DataError> {
    let mut last = 0;
    for row in rows {
        rise(file, &mut last, row.id)?;
        tables.entry(kind).or_default().push(Entry {
            id: row.id,
            kind,
            placeholder: false,
            name_en: row.name_en.clone(),
            name_fr: row.name_fr.clone(),
            ability: None,
        });
    }
    Ok(())
}
