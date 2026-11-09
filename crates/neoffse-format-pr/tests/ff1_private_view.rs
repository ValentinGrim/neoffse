//! Summary and view of the FF I model on the private slot files. The folder is
//! read-only input and the tests write nothing. A failure names the label and
//! a field path, never a value, a length or a name: the files hold personal
//! data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::exit_facts::{CHARS, INV, KEYS};
use common::ff1_real::slots;
use common::json_tree::Tree;
use common::private::assert_fields_read_back;
use common::real_slots::{must, must_some};
use neoffse_core::{Capability, FieldId};
use neoffse_data::Language;
use neoffse_format_pr::ff1::{Ff1Model, names};
use neoffse_format_pr::slots::slot_for_name;

#[test]
fn real_slots_have_a_summary() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model::<Ff1Model>();
        let Ok(summary) = model.summary() else {
            panic!("{}: the summary does not read", slot.label)
        };
        let rows = must(
            model.rows(&FieldId::new(names::CHARACTER)),
            &format!("{}: character rows do not read", slot.label),
        )
        .len();
        assert!(!summary.party.is_empty(), "{}: the party is empty", slot.label);
        assert!(summary.party.len() <= rows, "{}: the party is longer than the roster", slot.label);
        assert!(summary.play_time.is_some(), "{}: no play time", slot.label);
    }
}

#[test]
fn real_summary_slot_matches_the_file_name() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let Ok(summary) = slot.model::<Ff1Model>().summary() else {
            panic!("{}: the summary does not read", slot.label)
        };
        assert!(summary.slot.is_some(), "{}: no slot from the id", slot.label);
        assert!(summary.slot == slot_for_name(&slot.name), "{}: id and file name disagree", slot.label);
    }
}

#[test]
fn real_views_cover_every_row() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model::<Ff1Model>();
        let Ok(view) = model.view(Language::En) else {
            panic!("{}: the view does not read", slot.label)
        };
        assert!(view.sections.len() == 3, "{}: section count", slot.label);
        // The counts come from the JSON tree, not from the model that builds the view.
        let tree = Tree::parse(&slot.json);
        let count = |path| must_some(tree.array_len(path), &format!("{}: no array at {path}", slot.label));
        let characters = count(CHARS);
        let rows = count(INV) + count(KEYS);
        let party = must_some(view.section(Capability::Party), &format!("{}: no Party section", slot.label));
        let inventory = must_some(view.section(Capability::Inventory), &format!("{}: no Inventory section", slot.label));
        assert!(party.entities.len() == characters, "{}: party entities", slot.label);
        assert!(inventory.entities.len() == 1 + rows, "{}: inventory entities", slot.label);
        assert_fields_read_back(&slot.label, &mut model, party.entities.iter().chain(&inventory.entities));
    }
}
