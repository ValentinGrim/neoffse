//! A gil edit and a level increase saved through the editor over a copy of the
//! owner's FF I saves (`NEOFFSE_SAVES`, read only). Every slot is copied into a
//! temporary folder first, saved into it with a temporary data folder, and
//! compared with the original opened by the facade with the same two edits
//! applied through `Session::edit`. The growth rolls are not injectable: the
//! rolled fields are checked against the growth rule, every other field
//! against the reference. A failure names a slot number and a field id, never
//! a value, a file name or a path.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{
    Editing, Flow, Outcomes, Part, PrivateSlot, Step, assert_same, field, fields, must, must_some, pages, private_slot_copies, saving_over, session_in,
};
use neoffse::core::{Capability, Edit, FieldId, Game, Key, SaveView, Value};
use neoffse::data::{Language, tables_for};
use neoffse::io::dirs::DataDir;
use neoffse::pr::container;
use neoffse::pr::document::{Document, Scalar, Segment};
use neoffse::pr::ff1::names;
use neoffse::{Registry, Running};
use neoffse_app::editor::{Input, SaveOutcome};
use neoffse_app::home::OpenOutcome;

#[allow(clippy::print_stderr, reason = "the counts of the run are the only output a skipped-or-not test has")]
fn report(line: &str) {
    eprintln!("{line}");
}

fn int_of(view: &SaveView, field: &FieldId, what: &str) -> i64 {
    let (_, view) = must_some(view.find(field), what);
    match view.value {
        Value::Int(number) => number,
        _ => panic!("{what}"),
    }
}

/// The `characterStatusId` of the character at `index` of the party.
fn status_of(bytes: &[u8], index: usize) -> Option<u32> {
    let decoded = container::decode(bytes).ok()?;
    let mut document = Document::parse(&decoded.json).ok()?;
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
    match document.get(&path).ok()? {
        Scalar::Int(status) => u32::try_from(status).ok(),
        Scalar::Uint(status) => u32::try_from(status).ok(),
        _ => None,
    }
}

/// The first character below `max_level` whose job has a growth row for the
/// next level: `(party index, character id, level)`.
fn below_cap(view: &SaveView, max_level: i64) -> Option<(usize, u32, i64)> {
    let tables = tables_for(Game::Ff1Pr)?;
    let party = view.section(Capability::Party)?;
    party.entities.iter().enumerate().find_map(|(index, entity)| {
        let Some(Key::Id(id)) = entity.id.keys().last().copied() else { return None };
        let field = |name| FieldId::new(names::CHARACTER).with_id(id).with_name(name);
        let level = int_of(view, &field(names::LEVEL), "level");
        let job = u32::try_from(int_of(view, &field(names::JOB), "job")).ok()?;
        let next = u32::try_from(level + 1).ok()?;
        (level < max_level && tables.growth_step(job, next).is_some()).then_some((index, id, level))
    })
}

#[test]
fn every_private_slot_saves_a_gil_and_level_edit() {
    let Some(slots) = private_slot_copies() else { return };
    let tables = must_some(tables_for(Game::Ff1Pr), "no FF I tables");
    let max_level = must_some(tables.bound("MAX_LV"), "no MAX_LV in the FF I tables");
    let gil_id = FieldId::new(names::GIL);
    let (mut saved, mut leveled, mut checked) = (0, 0, 0);
    for (position, PrivateSlot { name, site, copy }) in slots.into_iter().enumerate() {
        let slot = position + 1;
        let original = must(fs::read(&copy), &format!("slot {slot}: the copy does not read"));
        let flow = Flow::new(&site, None);
        let outcomes = Outcomes::new();
        flow.click(&name, &outcomes);
        flow.manual.run_all();
        assert!(outcomes.outcomes() == vec![OpenOutcome::Opened], "slot {slot}: did not open");
        let saving = saving_over(Editing { flow }, copy.clone());

        let reference = must(tempfile::tempdir(), "no temporary folder");
        let reference_file = reference.path().join(&name);
        must(fs::write(&reference_file, &original), &format!("slot {slot}: the reference copy failed"));
        let mut reference_session = session_in(&reference_file);
        let before = must(reference_session.view(), &format!("slot {slot}: the original view failed"));

        let gil = int_of(&before, &gil_id, &format!("slot {slot}: gil"));
        let cap = must_some(before.find(&gil_id), "gil").1.meta.cap;
        let new_gil = if cap.is_none_or(|cap| gil < cap) { gil + 1 } else { gil - 1 };
        let pick = below_cap(&before, max_level);

        let editor = saving.ed.editor();
        let data = must(editor.data(), &format!("slot {slot}: data failed"));
        let gil_key = field(&data, Part::Inventory, "Gil", "Gil").key;
        assert!(
            editor.commit(gil_key, Input::Text(new_gil.to_string())).is_none(),
            "slot {slot}: the gil edit raised a notice"
        );
        must(
            reference_session.edit(&Edit::set(gil_id.clone(), Value::Int(new_gil))),
            &format!("slot {slot}: the reference gil edit failed"),
        );
        let mut step = None;
        if let Some((index, id, level)) = pick {
            let page = pages(&data.party).remove(index);
            let key = must_some(fields(&page).into_iter().find(|row| row.label == "Level"), "level row").key;
            assert!(
                editor.commit(key, Input::Text((level + 1).to_string())).is_none(),
                "slot {slot}: the level edit raised a notice"
            );
            let level_id = FieldId::new(names::CHARACTER).with_id(id).with_name(names::LEVEL);
            must(
                reference_session.edit(&Edit::set(level_id, Value::Int(level + 1))),
                &format!("slot {slot}: the reference level edit failed"),
            );
            let field = |name| FieldId::new(names::CHARACTER).with_id(id).with_name(name);
            step = Some(Step {
                tables,
                character: id,
                job: must(u32::try_from(int_of(&before, &field(names::JOB), "job")), "job"),
                base_vitality: must_some(
                    status_of(&original, index).and_then(|status| tables.base_vitality(status)),
                    &format!("slot {slot}: no base vitality"),
                ),
                vitality: int_of(&before, &field(names::VITALITY), "vitality"),
                new_level: must(u32::try_from(level + 1), "level"),
            });
            leveled += 1;
        }

        saving.save(Running::Refuse);
        saving.ed.flow.manual.run_all();
        assert!(matches!(saving.outcomes().as_slice(), [SaveOutcome::Saved { .. }]), "slot {slot}: not saved");
        saved += 1;

        let mut reopened = session_in(&copy);
        let after = must(reopened.view(), &format!("slot {slot}: the saved view failed"));
        let expected = must(reference_session.view(), &format!("slot {slot}: the reference view failed"));
        assert!(int_of(&after, &gil_id, "gil") == new_gil, "slot {slot}: the saved gil differs from the edit");
        checked += assert_same(slot, &after, &expected, &before, step.as_ref());

        let listing = must(
            Registry::new(Language::En).backups(&DataDir::at(saving.data.path()), Game::Ff1Pr),
            "listing failed",
        );
        assert!(listing.backups.len() == 1, "slot {slot}: not one backup");
        let backed_up = must(fs::read(listing.backups[0].dir.join(&name)), &format!("slot {slot}: the backup does not read"));
        assert!(backed_up == original, "slot {slot}: the backup differs from the original file");
    }
    report(&format!(
        "editor save private: {saved} slots saved, {leveled} took the level edit, {checked} rolled fields checked"
    ));
}
