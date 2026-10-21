//! The entries a picker offers.

use neoffse::core::TableKind;
use neoffse::data::{GameTables, Language};

use crate::PickerEntry;

/// The entries of `kinds` that match `query`.
///
/// `query` loses its surrounding ASCII whitespace first. An entry matches when
/// its name contains the rest (case-insensitive, Unicode lowercase) or when its
/// id equals the number it spells in ASCII digits. An empty or all-space
/// `query` matches every entry. The result is sorted by id with the
/// kinds merged; a kind the game has no table for adds nothing, and a kind
/// listed twice counts once. `skip_placeholders` drops the entries marked
/// placeholder. An id above `i32::MAX` is dropped: Slint holds an `int`.
#[must_use]
pub fn picker_entries(tables: &GameTables, kinds: &[TableKind], language: Language, query: &str, skip_placeholders: bool) -> Vec<PickerEntry> {
    let query = query.trim_ascii();
    let needle = query.to_lowercase();
    let wanted_id = spelled_id(query);
    let mut seen: Vec<TableKind> = Vec::new();
    let mut found: Vec<PickerEntry> = Vec::new();
    for kind in kinds {
        if seen.contains(kind) {
            continue;
        }
        seen.push(*kind);
        for entry in tables.table(*kind).unwrap_or_default() {
            if skip_placeholders && entry.placeholder {
                continue;
            }
            let Ok(id) = i32::try_from(entry.id) else { continue };
            let name = entry.name(language);
            if wanted_id == Some(entry.id) || name.to_lowercase().contains(&needle) {
                found.push(PickerEntry { id, name: name.into() });
            }
        }
    }
    found.sort_by_key(|entry| entry.id);
    found
}

/// The name of entry `id` in the first of `kinds` that holds it.
///
/// `None` for a negative or unknown id. The name can be empty when the table
/// gives none.
#[must_use]
pub fn entry_name(tables: &GameTables, kinds: &[TableKind], language: Language, id: i64) -> Option<String> {
    let id = u32::try_from(id).ok()?;
    kinds
        .iter()
        .find_map(|kind| tables.entry(*kind, id))
        .map(|entry| entry.name(language).to_owned())
}

/// The id a trimmed query of ASCII digits spells, `None` for anything else.
fn spelled_id(query: &str) -> Option<u32> {
    if query.is_empty() || !query.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    query.parse().ok()
}
