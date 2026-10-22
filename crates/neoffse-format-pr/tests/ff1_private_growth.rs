//! The computed `max_hp` and `next_level` fields on the private slot files.
//! The folder is read-only input and the tests write nothing. A failure names
//! the label, a character id and a field name, never a value, a length or a
//! name: the files hold personal data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_real::{RealSlot, must, must_some, slots};
use common::ff1_save::character_field;
use neoffse_core::{Capability, FieldId, Value};
use neoffse_data::Language;
use neoffse_format_pr::ff1::{GAME, names};

/// The play time that the menu shows as 12:28, in whole minutes.
const MENU_MINUTES: u64 = 748;
const MENU_GIL: i64 = 999_999;
/// Maximum HP and next-level figure of characters 1 to 4 on that menu.
const MENU_MAX_HP: [i64; 4] = [999, 863, 930, 679];
const MENU_NEXT_LEVEL: [i64; 4] = [20_295, 916, 11_681, 11_681];

fn is_menu_state(slot: &RealSlot) -> bool {
    let mut model = slot.model();
    let Ok(summary) = model.summary() else { return false };
    let minutes = summary.play_time.map(|time| time.as_secs() / 60);
    let gil = model.get(&FieldId::new(names::GIL)).ok();
    minutes == Some(MENU_MINUTES) && gil == Some(Value::Int(MENU_GIL))
}

#[test]
fn the_private_state_at_12_28_shows_the_menu_values() {
    let Some(slots) = slots() else { return };
    let candidates: Vec<RealSlot> = slots.into_iter().filter(is_menu_state).collect();
    if candidates.is_empty() {
        neoffse_testkit::skip("no private FF I slot at the 12:28 state");
        return;
    }
    for slot in candidates {
        let mut model = slot.model();
        let view = must(model.view(Language::En), &format!("{}: the view does not read", slot.label));
        for (index, (max_hp, next_level)) in MENU_MAX_HP.iter().zip(MENU_NEXT_LEVEL).enumerate() {
            let id = u32::try_from(index).unwrap() + 1;
            for (name, wanted) in [(names::MAX_HP, *max_hp), (names::NEXT_LEVEL, next_level)] {
                let found = must_some(
                    view.find(&character_field(id, name)),
                    &format!("{}: character {id} {name} is not in the view", slot.label),
                );
                assert!(
                    found.1.value == Value::Int(wanted),
                    "{}: character {id} {name} differs from the menu",
                    slot.label
                );
            }
        }
    }
}

#[test]
fn every_private_slot_shows_both_fields_in_range() {
    let Some(slots) = slots() else { return };
    let tables = must_some(neoffse_data::tables_for(GAME), "no FF I tables");
    let cap = must_some(tables.bound("MAX_HP"), "no MAX_HP cap");
    for slot in slots {
        let mut model = slot.model();
        let view = must(model.view(Language::En), &format!("{}: the view does not read", slot.label));
        let party = must_some(view.section(Capability::Party), &format!("{}: no Party section", slot.label));
        for entity in &party.entities {
            for name in [names::MAX_HP, names::NEXT_LEVEL] {
                let found = entity
                    .fields
                    .iter()
                    .find(|field| field.id.keys().last() == character_field(1, name).keys().last());
                let field = must_some(found, &format!("{}: {} lacks {name}", slot.label, entity.id));
                let Value::Int(number) = field.value else {
                    panic!("{}: {} {name} is no integer", slot.label, entity.id)
                };
                let in_range = if name == names::MAX_HP { (0..=cap).contains(&number) } else { number >= 1 };
                assert!(in_range, "{}: {} {name} is out of range", slot.label, entity.id);
                let read = must(model.get(&field.id), &format!("{}: {} {name} does not read", slot.label, entity.id));
                assert!(read == field.value, "{}: {} {name} differs from the model", slot.label, entity.id);
            }
        }
    }
}

#[test]
fn computing_the_fields_changes_no_private_slot() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model();
        must(model.view(Language::En), &format!("{}: the view does not read", slot.label));
        let mut again = slot.model();
        for id in 1..=4 {
            for name in [names::MAX_HP, names::NEXT_LEVEL] {
                // A character the slot lacks, or a field it leaves out, is no failure here.
                let _ = again.get(&character_field(id, name));
            }
        }
        assert!(model.document().to_json() == slot.json, "{}: the view changed the document", slot.label);
        assert!(again.document().to_json() == slot.json, "{}: a get changed the document", slot.label);
    }
}
