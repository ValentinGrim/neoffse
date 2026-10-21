//! The FF I spell slots on the private slot files. The folder is read-only
//! input and the tests write nothing. A failure names the label and a field
//! path, never a value: the files hold personal data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_magic::slot_mismatches;
use common::ff1_real::{must, slots};
use neoffse_core::{CapPolicy, Capability, Edit, EditError, FieldId, Value};
use neoffse_data::Language;
use neoffse_format_pr::ff1::names;

#[test]
fn real_spell_slots_equal_the_ability_list() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model();
        let found = slot_mismatches(&mut model, &slot.json);
        assert!(found.is_empty(), "{}: {}", slot.label, found.join("; "));
    }
}

#[test]
fn real_spell_slots_read_and_show_in_the_view() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model();
        let ids = must(
            model.rows(&FieldId::new(names::CHARACTER)),
            &format!("{}: character rows do not read", slot.label),
        );
        let view = must(model.view(Language::En), &format!("{}: the view does not read", slot.label));
        let Some(magic) = view.section(Capability::Magic) else {
            panic!("{}: no Magic section", slot.label)
        };
        assert!(magic.entities.len() == ids.len(), "{}: Magic entities", slot.label);
        for entity in &magic.entities {
            assert!(entity.fields.len() == 24, "{}: {} spell fields", slot.label, entity.id);
        }
    }
}

#[test]
fn real_spell_slots_refuse_every_edit() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model();
        let before = model.document().to_json();
        let ids = must(
            model.rows(&FieldId::new(names::CHARACTER)),
            &format!("{}: character rows do not read", slot.label),
        );
        for id in ids {
            for field in names::spell_fields(id) {
                let edit = Edit::set(field.clone(), Value::Int(5));
                let raw = model.apply(&edit, CapPolicy::StorageOnly);
                assert!(matches!(raw, Err(EditError::ReadOnly { .. })), "{}: {field} raw set", slot.label);
                let game = model.apply_game(&edit);
                assert!(matches!(game, Err(EditError::ReadOnly { .. })), "{}: {field} game set", slot.label);
            }
        }
        assert!(model.document().to_json() == before, "{}: a refused edit changed the document", slot.label);
    }
}
