//! Summary and view of the FF I model on the private slot files. The folder is
//! read-only input and the tests write nothing. A failure names the label and
//! a field path, never a value, a length or a name: the files hold personal
//! data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_real::slots;
use neoffse_core::{Capability, FieldId};
use neoffse_data::Language;
use neoffse_format_pr::ff1::names;
use neoffse_format_pr::slots::slot_for_name;

#[test]
fn real_slots_have_a_summary() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model();
        let Ok(summary) = model.summary() else {
            panic!("{}: the summary does not read", slot.label)
        };
        let rows = model.rows(&FieldId::new(names::CHARACTER)).unwrap().len();
        assert!(!summary.party.is_empty(), "{}: the party is empty", slot.label);
        assert!(summary.party.len() <= rows, "{}: the party is longer than the roster", slot.label);
        assert!(summary.play_time.is_some(), "{}: no play time", slot.label);
    }
}

#[test]
fn real_summary_slot_matches_the_file_name() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let Ok(summary) = slot.model().summary() else {
            panic!("{}: the summary does not read", slot.label)
        };
        assert!(summary.slot.is_some(), "{}: no slot from the id", slot.label);
        assert_eq!(summary.slot, slot_for_name(&slot.name), "{}: id and file name disagree", slot.label);
    }
}

#[test]
fn real_views_cover_every_row() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model();
        let Ok(view) = model.view(Language::En) else {
            panic!("{}: the view does not read", slot.label)
        };
        assert_eq!(view.sections.len(), 2, "{}: section count", slot.label);
        let count = |model: &mut neoffse_format_pr::ff1::Ff1Model, name| model.rows(&FieldId::new(name)).unwrap().len();
        let characters = count(&mut model, names::CHARACTER);
        let rows = count(&mut model, names::INVENTORY) + count(&mut model, names::KEY_ITEMS);
        let party = view.section(Capability::Party).unwrap();
        let inventory = view.section(Capability::Inventory).unwrap();
        assert_eq!(party.entities.len(), characters, "{}: party entities", slot.label);
        assert_eq!(inventory.entities.len(), 1 + rows, "{}: inventory entities", slot.label);
        for entity in party.entities.iter().chain(&inventory.entities) {
            for field in &entity.fields {
                let Ok(value) = model.get(&field.id) else {
                    panic!("{}: {} does not read", slot.label, field.id)
                };
                assert_eq!(value, field.value, "{}: {} differs from the model", slot.label, field.id);
            }
        }
    }
}
