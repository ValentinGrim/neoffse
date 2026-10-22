//! The growth step on the private slot files, read only: every case works on
//! an in-memory model. A failure names the label, a character id and a
//! field name, never a value, a length or a name: the files hold personal
//! data (`docs/architecture.md#test-policy`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_real::{RealSlot, must, must_some, slots};
use common::ff1_save::character_field;
use common::growth_oracle::{Stored, grow};
use common::rolls::Script;
use neoffse_core::{CapPolicy, ChangeSet, Edit, FieldId, Value};
use neoffse_data::GameTables;
use neoffse_format_pr::document::{Scalar, Segment};
use neoffse_format_pr::ff1::{Ff1Model, GAME, names};

/// A character as the growth step reads it.
struct Facts {
    id: u32,
    level: u32,
    job: u32,
    base_vitality: u32,
}

fn tables() -> &'static GameTables {
    must_some(neoffse_data::tables_for(GAME), "no FF I tables")
}

fn int(model: &mut Ff1Model, slot: &RealSlot, id: u32, name: &'static str) -> i64 {
    match model.get(&character_field(id, name)) {
        Ok(Value::Int(value)) => value,
        _ => panic!("{}: character {id} {name} does not read", slot.label),
    }
}

/// The characters of the slot with a level below 97, so that a rise of 3 stays
/// inside the table.
fn characters(slot: &RealSlot, model: &mut Ff1Model) -> Vec<Facts> {
    let rows = must(model.rows(&FieldId::new(names::CHARACTER)), &format!("{}: no character list", slot.label));
    let mut found = Vec::new();
    for (index, id) in rows.into_iter().enumerate() {
        let path = [
            Segment::Key("userData"),
            Segment::Doc,
            Segment::Key("ownedCharacterList"),
            Segment::Doc,
            Segment::Key("target"),
            Segment::Index(index),
            Segment::Doc,
            Segment::Key("characterStatusId"),
        ];
        let Ok(Scalar::Int(status)) = model.read(&path) else {
            panic!("{}: character {id} characterStatusId does not read", slot.label)
        };
        let level = u32::try_from(int(model, slot, id, names::LEVEL)).unwrap_or(u32::MAX);
        let job = u32::try_from(int(model, slot, id, names::JOB)).unwrap_or(u32::MAX);
        let base = u32::try_from(status).ok().and_then(|status| tables().base_vitality(status));
        let base_vitality = must_some(base, &format!("{}: character {id} has no base vitality", slot.label));
        if level < 97 {
            found.push(Facts { id, level, job, base_vitality });
        }
    }
    found
}

fn level_edit(id: u32, level: u32) -> Edit {
    Edit::set(character_field(id, names::LEVEL), Value::Int(i64::from(level)))
}

#[test]
fn the_extremes_match_the_oracle() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut probe = slot.model();
        for facts in characters(&slot, &mut probe) {
            let to = facts.level + 3;
            for (succeed, mut script) in [(false, Script::fixed(false, 20)), (true, Script::fixed(true, 24))] {
                let mut model = slot.model();
                let start = Stored::read(&mut model, facts.id);
                let wanted = grow(tables(), &start, facts.base_vitality, facts.job, facts.level, to, succeed);
                must(
                    model.apply_game_with(&level_edit(facts.id, to), &mut script),
                    &format!("{}: character {} refused the level edit", slot.label, facts.id),
                );
                let got = Stored::read(&mut model, facts.id);
                assert!(
                    got == wanted,
                    "{}: character {} differs from the oracle (all rolls {succeed})",
                    slot.label,
                    facts.id
                );
            }
        }
    }
}

fn apply_all(model: &mut Ff1Model, set: &ChangeSet) {
    must(
        set.apply_with(CapPolicy::StorageOnly, |step, policy| model.apply(step, policy)),
        "a replay refused an edit",
    );
}

#[test]
fn a_private_level_up_undoes_byte_for_byte() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut probe = slot.model();
        for facts in characters(&slot, &mut probe) {
            let mut model = slot.model();
            let before = model.document().to_json();
            let change = must(
                model.apply_game_with(&level_edit(facts.id, facts.level + 3), &mut Script::fixed(true, 24)),
                &format!("{}: character {} refused the level edit", slot.label, facts.id),
            );
            let after = model.document().to_json();
            apply_all(&mut model, &change.inverse);
            assert!(model.document().to_json() == before, "{}: character {} undo differs", slot.label, facts.id);
            apply_all(&mut model, &change.forward);
            assert!(model.document().to_json() == after, "{}: character {} redo differs", slot.label, facts.id);
        }
    }
}

#[test]
fn a_private_level_decrease_changes_no_stat_and_draws_nothing() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut probe = slot.model();
        for facts in characters(&slot, &mut probe).into_iter().filter(|facts| facts.level > 1) {
            let mut model = slot.model();
            let before = Stored::read(&mut model, facts.id);
            let mut script = Script::queued(&[], &[]);
            must(
                model.apply_game_with(&level_edit(facts.id, facts.level - 1), &mut script),
                &format!("{}: character {} refused the decrease", slot.label, facts.id),
            );
            assert!(
                Stored::read(&mut model, facts.id) == before,
                "{}: character {} moved a stat",
                slot.label,
                facts.id
            );
            assert_eq!(script.drawn(), (0, 0), "{}: character {} drew", slot.label, facts.id);
        }
    }
}

#[test]
fn the_real_source_lands_between_the_extremes() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut probe = slot.model();
        for facts in characters(&slot, &mut probe) {
            let to = facts.level + 1;
            let mut model = slot.model();
            let start = Stored::read(&mut model, facts.id);
            let low = grow(tables(), &start, facts.base_vitality, facts.job, facts.level, to, false);
            let high = grow(tables(), &start, facts.base_vitality, facts.job, facts.level, to, true);
            must(
                model.apply_game(&level_edit(facts.id, to)),
                &format!("{}: character {} refused the level edit", slot.label, facts.id),
            );
            let got = Stored::read(&mut model, facts.id);
            for ((field, value), ((_, low), (_, high))) in got
                .fields(facts.id)
                .into_iter()
                .zip(low.fields(facts.id).into_iter().zip(high.fields(facts.id)))
            {
                assert!(
                    (low..=high).contains(&value),
                    "{}: character {} {field} outside the extremes",
                    slot.label,
                    facts.id
                );
            }
        }
    }
}
