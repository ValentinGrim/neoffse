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
use crate::parse::{rise, rule};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentFile {
    content: Vec<ContentRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentRow {
    id: u32,
    kind: String,
    placeholder: Option<bool>,
    name_en: Option<String>,
    name_fr: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AbilityFile {
    ability: Vec<AbilityRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AbilityRow {
    id: u32,
    content_id: u32,
    kind: String,
    level: u8,
    name_en: Option<String>,
    name_fr: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NameRow {
    id: u32,
    name_en: Option<String>,
    name_fr: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JobFile {
    pub(crate) job: Vec<NameRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommandFile {
    pub(crate) command: Vec<NameRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MonsterFile {
    pub(crate) monster: Vec<NameRow>,
}

/// The two names of a row: both present or both absent
/// (`docs/formats/pixel-remaster.md#rows-without-a-name`). An empty string is a name.
fn name_pair(file: &'static str, id: u32, en: Option<&str>, fr: Option<&str>) -> Result<(Option<String>, Option<String>), DataError> {
    match (en, fr) {
        (Some(_), Some(_)) | (None, None) => Ok((en.map(str::to_owned), fr.map(str::to_owned))),
        _ => Err(rule(file, format!("id {id}"), "name in one language only")),
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

pub(crate) fn content(file: &ContentFile, tables: &mut BTreeMap<TableKind, Vec<Entry>>) -> Result<(), DataError> {
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
        let (name_en, name_fr) = name_pair(NAME, row.id, row.name_en.as_deref(), row.name_fr.as_deref())?;
        tables.entry(kind).or_default().push(Entry {
            id: row.id,
            kind,
            placeholder,
            name_en,
            name_fr,
            ability: None,
        });
    }
    Ok(())
}

pub(crate) fn abilities(file: &AbilityFile, tables: &mut BTreeMap<TableKind, Vec<Entry>>) -> Result<(), DataError> {
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
        let (name_en, name_fr) = name_pair(NAME, row.id, row.name_en.as_deref(), row.name_fr.as_deref())?;
        tables.entry(TableKind::Ability).or_default().push(Entry {
            id: row.id,
            kind: TableKind::Ability,
            placeholder: false,
            name_en,
            name_fr,
            ability: Some(AbilityInfo {
                content_id: row.content_id,
                kind,
                level: row.level,
            }),
        });
    }
    Ok(())
}

pub(crate) fn names(file: &'static str, rows: &[NameRow], kind: TableKind, tables: &mut BTreeMap<TableKind, Vec<Entry>>) -> Result<(), DataError> {
    let mut last = 0;
    for row in rows {
        rise(file, &mut last, row.id)?;
        let (name_en, name_fr) = name_pair(file, row.id, row.name_en.as_deref(), row.name_fr.as_deref())?;
        tables.entry(kind).or_default().push(Entry {
            id: row.id,
            kind,
            placeholder: false,
            name_en,
            name_fr,
            ability: None,
        });
    }
    Ok(())
}
