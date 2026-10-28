//! The editor data over a copy of the owner's FF I saves (`NEOFFSE_SAVES`,
//! read only). Every slot is copied into a temporary folder first. The test
//! skips when the folder is absent. A failure names a slot number and a field
//! label, never a value, a file name or a path. No edit, no save: the copy
//! keeps its bytes.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::sync::Arc;

use common::{Flow, Outcomes, Part, PrivateSlot, all_fields, fields, items, lists, must, pages, private_slot_copies, rows, section};
use neoffse::core::{Key, RowId};
use neoffse::data::Language;
use neoffse_app::home::OpenOutcome;
use neoffse_app::{EditorData, EditorViewModel, FieldKind};

/// The counts that describe the shape of the data: pages, fields per page,
/// lists, rows per list.
fn shape(data: &EditorData) -> Vec<usize> {
    let mut out = Vec::new();
    for part in [Part::Party, Part::Inventory, Part::Magic] {
        let section = section(data, part);
        out.push(usize::from(section.present));
        for page in pages(&section) {
            out.push(fields(&page).len());
        }
        for list in lists(&section) {
            out.push(rows(&list).len());
        }
    }
    out
}

fn check(slot: usize, data: &EditorData, editor: &EditorViewModel) {
    assert!(
        data.party.present && data.inventory.present && data.magic.present,
        "slot {slot}: a section is absent"
    );
    for page in pages(&data.magic) {
        for row in fields(&page) {
            assert!(row.kind == FieldKind::Label, "slot {slot}: magic field {} is not a label", row.label);
            assert!(!row.editable, "slot {slot}: magic field {} is editable", row.label);
        }
    }
    let all = all_fields(data);
    let keys: Vec<i32> = all.iter().map(|row| row.key).collect();
    assert!(
        keys == (0..i32::try_from(keys.len()).unwrap()).collect::<Vec<_>>(),
        "slot {slot}: keys are not dense"
    );
    for row in all.iter().filter(|row| row.kind == FieldKind::Picker) {
        assert!(
            !editor.field_entries(row.key, "").is_empty(),
            "slot {slot}: picker {} offers no entry",
            row.label
        );
    }
    for list in lists(&data.inventory) {
        assert!(list.insertable, "slot {slot}: list {} is not insertable", list.title);
        assert!(
            !editor.list_entries(list.key, "").is_empty(),
            "slot {slot}: list {} offers no entry",
            list.title
        );
    }
    assert!(items(&data.party.lists).is_empty(), "slot {slot}: the party has a list");
}

#[test]
fn every_private_slot_builds_the_editor_data() {
    let Some(slots) = private_slot_copies() else { return };
    for (position, PrivateSlot { name, site, copy }) in slots.iter().enumerate() {
        let index = position + 1;
        let bytes = must(fs::read(copy), &format!("private slot {index}: the copy does not read"));

        let flow = Flow::new(site, None);
        let outcomes = Outcomes::new();
        flow.click(name, &outcomes);
        flow.manual.run_all();
        assert!(outcomes.outcomes() == vec![OpenOutcome::Opened], "private slot {index} did not open");

        let data = flow.editor.data().unwrap_or_else(|_| panic!("private slot {index}: data failed"));
        check(index, &data, &flow.editor);

        let session = flow.state.session().unwrap();
        {
            let mut session = session.lock().unwrap();
            let view = must(session.view(), &format!("private slot {index}: the view failed"));
            for section in &view.sections {
                for entity in &section.entities {
                    for field in &entity.fields {
                        session
                            .field_locked(&field.id)
                            .unwrap_or_else(|_| panic!("private slot {index}: lock query failed on {}", field.meta.label));
                    }
                    let parent = entity.id.parent();
                    let Some(Key::Id(key)) = entity.id.keys().last().copied() else { continue };
                    for collection in section.collections.iter().filter(|meta| parent.as_ref() == Some(&meta.id)) {
                        session
                            .row_locked(&RowId::new(collection.id.clone(), key))
                            .unwrap_or_else(|_| panic!("private slot {index}: row lock query failed in {}", collection.label));
                    }
                }
            }
        }

        let default_shape = shape(&data);
        for expert in [true, false] {
            flow.editor.set_expert(expert);
            let rebuilt = flow
                .editor
                .data()
                .unwrap_or_else(|_| panic!("private slot {index}: expert {expert} data failed"));
            check(index, &rebuilt, &flow.editor);
            assert!(shape(&rebuilt) == default_shape, "private slot {index}: the shape changes with the mode");
        }

        let french = EditorViewModel::new(Arc::clone(&flow.state), Language::Fr);
        let french_data = french.data().unwrap_or_else(|_| panic!("private slot {index}: French data failed"));
        check(index, &french_data, &french);
        assert!(
            shape(&french_data) == default_shape,
            "private slot {index}: the shape changes with the language"
        );

        assert!(!flow.editor.is_dirty(), "private slot {index} is dirty");
        assert!(
            must(fs::read(copy), &format!("private slot {index}: the copy does not read")) == bytes,
            "private slot {index}: the copy changed"
        );
    }
}
