//! `Insert`, `Remove` and `row_locked` on the private slot files. The folder
//! is read-only input: edits happen in memory and the tests write nothing.
//! A failure names the label and a path, never a value.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_real::{RealSlot, slots};
use neoffse_core::{CapPolicy, Edit, FieldId, RowId, TableKind, Value};
use neoffse_format_pr::container::{decode, encode};
use neoffse_format_pr::ff1::{GAME, names};

const ENFORCE: CapPolicy = CapPolicy::Enforce;
const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;

fn inventory() -> FieldId {
    FieldId::new(names::INVENTORY)
}

/// The first non-placeholder `Item` table id that `rows` lacks.
fn free_item(rows: &[u32]) -> Option<u32> {
    let table = neoffse_data::tables_for(GAME)?.table(TableKind::Item)?;
    table.iter().find(|entry| !entry.placeholder && !rows.contains(&entry.id)).map(|entry| entry.id)
}

#[test]
fn real_insert_survives_encode_and_decode() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let label = &slot.label;
        let mut model = slot.model();
        let old = model.rows(&inventory()).unwrap_or_else(|_| panic!("{label}: inventory rows do not read"));
        let Some(id) = free_item(&old) else { panic!("{label}: no free item id") };
        let row = RowId::new(inventory(), id);
        let Ok(inverse) = model.apply(&Edit::insert(row.clone(), None, Vec::new()), ENFORCE) else {
            panic!("{label}: the insert is refused")
        };
        let Ok(decoded) = decode(&slot.bytes) else { panic!("{label}: decode failed") };
        let Ok(bytes) = encode(&model.document().to_json(), &decoded.container) else {
            panic!("{label}: encode failed")
        };
        let Ok(again) = decode(&bytes) else { panic!("{label}: second decode failed") };
        let mut rebuilt = RealSlot {
            label: label.clone(),
            bytes,
            json: again.json,
        }
        .model();
        let mut want = old.clone();
        want.push(id);
        assert_eq!(rebuilt.rows(&inventory()).unwrap(), want, "{label}: rows after the round trip");
        let count = rebuilt.get(&inventory().with_id(id).with_name(names::COUNT));
        assert_eq!(count.unwrap(), Value::Int(1), "{label}: new row count");
        let Ok(_) = rebuilt.apply(&inverse, STORAGE_ONLY) else {
            panic!("{label}: the inverse is refused")
        };
        assert_eq!(rebuilt.document().to_json(), slot.json, "{label}: undo");
    }
}

#[test]
fn real_remove_and_undo_restore_the_text() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let label = &slot.label;
        let mut model = slot.model();
        let rows = model.rows(&inventory()).unwrap_or_else(|_| panic!("{label}: inventory rows do not read"));
        let mut picked = None;
        for id in rows.iter().rev() {
            let row = RowId::new(inventory(), *id);
            let Ok(locked) = model.row_locked(&row) else {
                panic!("{label}: row_locked fails on a row of the list")
            };
            if !locked {
                picked = Some(row);
                break;
            }
        }
        let Some(row) = picked else {
            neoffse_testkit::skip(&format!("{label} has no unlocked inventory row"));
            continue;
        };
        let Ok(inverse) = model.apply(&Edit::remove(row), ENFORCE) else {
            panic!("{label}: an unlocked row is refused")
        };
        assert_ne!(model.document().to_json(), slot.json, "{label}: the remove changed nothing");
        let Ok(_) = model.apply(&inverse, STORAGE_ONLY) else {
            panic!("{label}: the inverse is refused")
        };
        assert_eq!(model.document().to_json(), slot.json, "{label}: undo");
    }
}

#[test]
fn real_rows_keep_order_after_an_insert_in_the_middle() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let label = &slot.label;
        let mut model = slot.model();
        let old = model.rows(&inventory()).unwrap_or_else(|_| panic!("{label}: inventory rows do not read"));
        let Some(id) = free_item(&old) else { panic!("{label}: no free item id") };
        let middle = old.len() / 2;
        let row = RowId::new(inventory(), id);
        let Ok(inverse) = model.apply(&Edit::insert(row, Some(middle), Vec::new()), ENFORCE) else {
            panic!("{label}: the insert is refused")
        };
        let mut want = old.clone();
        want.insert(middle, id);
        assert_eq!(model.rows(&inventory()).unwrap(), want, "{label}: order after the insert");
        let Ok(_) = model.apply(&inverse, STORAGE_ONLY) else {
            panic!("{label}: the inverse is refused")
        };
        assert_eq!(model.document().to_json(), slot.json, "{label}: undo");
    }
}

#[test]
fn real_placeholder_and_worn_rows_are_locked() {
    let Some(slots) = slots() else { return };
    let Some(tables) = neoffse_data::tables_for(GAME) else {
        panic!("no FF I tables")
    };
    for slot in slots {
        let label = &slot.label;
        let mut model = slot.model();
        let rows = model.rows(&inventory()).unwrap_or_else(|_| panic!("{label}: inventory rows do not read"));
        for id in &rows {
            if tables.content(*id).is_some_and(|entry| entry.placeholder) {
                let locked = model.row_locked(&RowId::new(inventory(), *id));
                assert!(locked.unwrap(), "{label}: a placeholder row is not locked");
            }
        }
        let characters = model.rows(&FieldId::new(names::CHARACTER)).unwrap();
        for character in characters {
            for number in 1..=names::EQUIPMENT_SLOTS {
                let field = FieldId::new(names::CHARACTER).with_id(character).with_name(names::EQUIPMENT).with_id(number);
                let Ok(Value::Int(content)) = model.get(&field) else { continue };
                let Ok(content) = u32::try_from(content) else { continue };
                if rows.contains(&content) {
                    let locked = model.row_locked(&RowId::new(inventory(), content));
                    assert!(locked.unwrap(), "{label}: a worn row at {field} is not locked");
                }
            }
        }
    }
}
