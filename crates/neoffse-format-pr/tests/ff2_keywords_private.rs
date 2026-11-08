//! The keywords on the private FF II slot files. The folder is read-only
//! input and the tests write nothing. A failure names the label and a
//! collection, never a value, a length or a name: the files hold personal
//! data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_real::must;
use common::ff2_real::{model, slots};
use neoffse_core::{CapPolicy, Capability, Edit, EditError, FieldId, RowId, TableKind};
use neoffse_data::Language;
use neoffse_format_pr::ff2::names;

fn collection() -> FieldId {
    FieldId::new(names::KEYWORDS)
}

#[test]
fn every_real_keyword_is_a_row_of_the_keyword_table() {
    let Some(slots) = slots() else { return };
    let tables = neoffse_data::tables_for(neoffse_format_pr::ff2::GAME).unwrap();
    for slot in slots {
        let mut model = model(&slot);
        let keys = must(model.rows(&collection()), &format!("{}: keywords do not read", slot.label));
        for (index, key) in keys.iter().enumerate() {
            assert!(
                tables.entry(TableKind::Keyword, *key).is_some(),
                "{}: keywords: element {index} is no type-6 row",
                slot.label
            );
            assert_eq!(
                keys.iter().filter(|other| *other == key).count(),
                1,
                "{}: keywords: element {index} repeats",
                slot.label
            );
        }
    }
}

#[test]
fn every_real_keyword_row_is_locked_and_listed_in_the_view() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = model(&slot);
        let keys = must(model.rows(&collection()), &format!("{}: keywords do not read", slot.label));
        for (index, key) in keys.iter().enumerate() {
            let locked = must(
                model.row_locked(&RowId::new(collection(), *key)),
                &format!("{}: keywords: row {index} does not lock", slot.label),
            );
            assert!(locked, "{}: keywords: row {index} is not locked", slot.label);
        }
        let view = must(model.view(Language::En), &format!("{}: the view does not build", slot.label));
        let section = view.section(Capability::Inventory).unwrap();
        let listed = section.entities.iter().filter(|entity| entity.id.starts_with(&collection())).count();
        assert_eq!(listed, keys.len(), "{}: keywords: the view lists another number of rows", slot.label);
        assert!(
            section.collections.iter().any(|meta| meta.id == collection()),
            "{}: keywords: no collection meta",
            slot.label
        );
    }
}

#[test]
fn a_real_slot_refuses_a_game_removal_and_takes_a_raw_one_back() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = model(&slot);
        let keys = must(model.rows(&collection()), &format!("{}: keywords do not read", slot.label));
        let Some(first) = keys.first() else { continue };
        let row = RowId::new(collection(), *first);
        let error = model.apply_game(&Edit::remove(row.clone())).unwrap_err();
        assert!(
            matches!(error, EditError::ReadOnly { .. }),
            "{}: keywords: the game removal is not refused",
            slot.label
        );
        let inverse = must(
            model.apply(&Edit::remove(row), CapPolicy::StorageOnly),
            &format!("{}: keywords: the raw removal fails", slot.label),
        );
        must(
            model.apply(&inverse, CapPolicy::StorageOnly),
            &format!("{}: keywords: the inverse fails", slot.label),
        );
        let after = must(model.rows(&collection()), &format!("{}: keywords do not read after the undo", slot.label));
        assert_eq!(after, keys, "{}: keywords: the undo changes the order", slot.label);
    }
}
