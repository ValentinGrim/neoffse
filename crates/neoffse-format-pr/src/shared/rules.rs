//! The game rules over the item lists and the equipment slots that every
//! Pixel Remaster game shares: placeholder rows, the wearer count, the
//! storage bound of a count, the floor under a count, the rules of an
//! `Insert` and of a `Remove`, the lock of a row and the equip plan
//! (`docs/formats/pixel-remaster.md#inventory`,
//! `docs/formats/pixel-remaster.md#equipment-slots`).
//!
//! A game passes the number of its equipment slots. Each function reads the
//! document and the tables it is given and writes nothing.

use neoffse_core::{CapPolicy, Edit, EditError, FieldId, FieldMeta, RowId, Shape, Value};
use neoffse_data::GameTables;

use super::items::Lists;
use super::meta::{STORAGE_MAX, collection_meta};
use super::names;
use super::paths::{Collection, List, characters_target, element, with};
use super::read::{character_part, count_path, find_row, format_error, read_int, read_key, slot_path};
use crate::document::{Document, DocumentError, Segment};

/// A placeholder row of `inventory`: its count has no known meaning, so
/// the game interface leaves the row alone
/// (`docs/formats/pixel-remaster.md#inventory`). `key_items` has none.
pub(crate) fn is_placeholder(tables: &GameTables, list: List, key: u32) -> bool {
    list == List::Inventory && tables.content(key).is_some_and(|entry| entry.placeholder)
}

/// The number of equipment slots, over all characters, that hold
/// `content_id` (`docs/formats/pixel-remaster.md#inventory`: an equipped id
/// appears in the inventory with a count at least this number). `slots` is
/// the number of slots of the game: the keys 1 to `slots` count.
pub(crate) fn wearers(document: &mut Document, content_id: u32, slots: u32) -> Result<usize, DocumentError> {
    let target = characters_target();
    let mut count = 0;
    for index in 0..document.array_len(&target)? {
        let dictionary = with(element(target.clone(), index), &[Segment::Key("equipmentList"), Segment::Doc]);
        let keys = with(dictionary.clone(), &[Segment::Key("keys")]);
        for entry in 0..document.array_len(&keys)? {
            let slot = read_key(document, &with(keys.clone(), &[Segment::Index(entry)]))?;
            if !(1..=slots).contains(&slot) {
                continue;
            }
            let value = [Segment::Key("values"), Segment::Index(entry), Segment::Doc, Segment::Key("contentId")];
            if read_key(document, &with(dictionary.clone(), &value))? == content_id {
                count += 1;
            }
        }
    }
    Ok(count)
}

/// The number of equipment slots that hold `content_id`. A key item is
/// never worn, so a `key_items` row that goes through this check always
/// passes it.
pub(crate) fn wearers_of(document: &mut Document, content_id: u32, slots: u32, field: &FieldId) -> Result<usize, EditError> {
    wearers(document, content_id, slots).map_err(|source| format_error(field, source))
}

/// The upper storage bound of a count field.
pub(crate) fn storage_max(meta: &FieldMeta) -> i64 {
    match meta.shape {
        Shape::Int { max, .. } => max,
        _ => STORAGE_MAX,
    }
}

/// `OutOfRange { min: worn }` for a count below the wearers.
pub(crate) fn check_floor(field: &FieldId, count: i64, worn: usize, count_meta: &FieldMeta) -> Result<(), EditError> {
    let min = i64::try_from(worn).unwrap_or(i64::MAX);
    if count < min {
        return Err(EditError::OutOfRange {
            field: field.clone(),
            value: count,
            min,
            max: storage_max(count_meta),
        });
    }
    Ok(())
}

/// The lock of a `Remove` in the game interface: `ReadOnly` for a placeholder
/// row of `inventory` and for a row whose id a character wears. A row of
/// another collection passes.
pub(crate) fn check_remove(document: &mut Document, tables: &GameTables, row: &RowId, slots: u32) -> Result<(), EditError> {
    if let Some(Collection::Items(list)) = Collection::of(&row.collection()) {
        let locked = is_placeholder(tables, list, row.key()) || wearers_of(document, row.key(), slots, row.id())? > 0;
        if locked {
            return Err(EditError::ReadOnly { field: row.id().clone() });
        }
    }
    Ok(())
}

/// An `Insert` into `inventory` or `key_items`: a placeholder key is
/// `ReadOnly`, a key-item `count` other than 1 and a `count` below the
/// wearers are `OutOfRange`.
pub(crate) fn check_insert_rules(
    document: &mut Document,
    tables: &GameTables,
    row: &RowId,
    values: &[(&'static str, Value)],
    slots: u32,
) -> Result<(), EditError> {
    let collection = row.collection();
    let Some(kind @ Collection::Items(list)) = Collection::of(&collection) else {
        return Ok(());
    };
    let meta = collection_meta(kind, tables);
    if is_placeholder(tables, list, row.key()) {
        return Err(EditError::ReadOnly { field: row.id().clone() });
    }
    let given = values.iter().find(|(name, _)| *name == names::COUNT).map(|(_, value)| value);
    if let (List::KeyItems, Some(Value::Int(count))) = (list, given)
        && *count != 1
    {
        return Err(EditError::OutOfRange {
            field: row.field(names::COUNT),
            value: *count,
            min: 1,
            max: 1,
        });
    }
    meta.check_insert(row, values, CapPolicy::Enforce)?;
    let Some(count_field) = meta.fields.iter().find(|field| field.name == names::COUNT) else {
        return Ok(());
    };
    if let Some(Value::Int(count)) = given.or(Some(&count_field.initial)) {
        let worn = wearers_of(document, row.key(), slots, row.id())?;
        check_floor(&row.field(names::COUNT), *count, worn, &count_field.meta)?;
    }
    Ok(())
}

/// True when the game interface refuses to remove `row` whatever the values
/// are: a `character` row (the collection is not removable), a placeholder
/// row of `inventory`, a row whose id a character wears, or a row whose
/// stored token is not canonical, or a canonical row whose stored `count`
/// leaves the storage bound of a count.
///
/// # Errors
///
/// `NotACollection` for a row of another path, `MissingRow` for a row the
/// document does not hold (a character row included), `Format` when a read
/// fails.
pub(crate) fn row_locked(lists: &mut Lists<'_>, row: &RowId, slots: u32) -> Result<bool, EditError> {
    let (meta, collection) = lists.row_collection(row)?;
    let index = lists.row_index(row, collection)?;
    let Collection::Items(list) = collection else {
        return Ok(true);
    };
    if is_placeholder(lists.tables, list, row.key()) || wearers_of(lists.document, row.key(), slots, row.id())? > 0 {
        return Ok(true);
    }
    let Some(count) = lists.canonical_row(row, list, index)? else {
        return Ok(true);
    };
    let values = [(names::COUNT, Value::Int(count))];
    Ok(meta.check_insert(row, &values, CapPolicy::StorageOnly).is_err())
}

/// The count of the inventory row `content_id`, read for the equip plan.
fn inventory_count(document: &mut Document, content_id: u32, count_field: &FieldId) -> Result<i64, EditError> {
    let path = count_path(document, List::Inventory, content_id, count_field)?;
    read_int(document, &path).map_err(|source| format_error(count_field, source))
}

/// The `contentId` held by `slot` of character `character`, read for the
/// equip plan.
fn slot_content(document: &mut Document, character: u32, slot: u32, slot_field: &FieldId) -> Result<i64, EditError> {
    let path = character_part(document, character, slot_field, |document, base| slot_path(document, base, slot, false))?;
    read_int(document, &path).map_err(|source| format_error(slot_field, source))
}

/// The equip plan of `Set character.<id>.equipment.<slot>` to `content_id`:
/// the slot `contentId`, then an inventory `Insert` with the wearers as
/// `count` or a raise of the `count` to the wearers (nothing when the count
/// already meets them), then the slot `count` set to the inventory count.
/// Equipping the placeholder of the slot is the unequip: the first and third
/// steps only, the slot `count` copies the count of the placeholder row
/// (omitted when the row is absent).
pub(crate) fn equip_plan(
    document: &mut Document,
    tables: &GameTables,
    slot_field: &FieldId,
    (character, slot): (u32, u32),
    content_id: u32,
    slots: u32,
) -> Result<Vec<Edit>, EditError> {
    let mut plan = vec![Edit::set(slot_field.clone(), Value::Int(i64::from(content_id)))];
    let slot_count = slot_field.clone().with_name(names::COUNT);
    let row = RowId::new(FieldId::new(names::INVENTORY), content_id);
    let count_field = row.field(names::COUNT);
    let failed = |source| format_error(&count_field, source);
    let held = find_row(document, Collection::Items(List::Inventory), content_id).map_err(failed)?;
    let placeholder = is_placeholder(tables, List::Inventory, content_id);
    if placeholder {
        if held.is_some() {
            let count = inventory_count(document, content_id, &count_field)?;
            plan.push(Edit::set(slot_count, Value::Int(count)));
        }
        return Ok(plan);
    }
    let old = slot_content(document, character, slot, slot_field)?;
    let others = wearers_of(document, content_id, slots, slot_field)?.saturating_sub(usize::from(old == i64::from(content_id)));
    let needed = i64::try_from(others + 1).unwrap_or(i64::MAX);
    let copied = if held.is_none() {
        plan.push(Edit::insert(row, None, vec![(names::COUNT, Value::Int(needed))]));
        needed
    } else {
        let stored = inventory_count(document, content_id, &count_field)?;
        if stored < needed {
            plan.push(Edit::set(count_field, Value::Int(needed)));
            needed
        } else {
            stored
        }
    };
    plan.push(Edit::set(slot_count, Value::Int(copied)));
    Ok(plan)
}
