//! The FF II exit proof on every private slot: the empty edit that changes
//! no byte of the document, the edit matrix, the Level proof that must fail
//! once the dictionary copy leaves its expected set, then the refusals that
//! must write nothing. The saves are read-only input and the tests write
//! nothing to disk. A failure names the slot label, the case, the interface
//! and a path or a check, never a value, a length or a name: the files hold
//! personal data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};

use common::edit_proof::Subject;
use common::exit_ff2::{ALWAYS, build, level_without_copy, run};
use common::ff2_real::slots;
use common::private::{inventory_ids, is_placeholder, is_read_only, refuses, skipped_line, worn_ids};
use common::real_slots::{must, must_some};
use neoffse_core::{CapPolicy, Capability, ChangeSet, Edit, EditError, FieldId, Key, RowId, Value};
use neoffse_format_pr::ff2::{Ff2Format, Ff2Model, names};
use neoffse_testkit::report;

#[test]
fn real_slots_prove_every_edit_kind() {
    let Some(slots) = slots() else { return };
    let (mut cases_run, mut runs, mut spells) = (0, 0, 0);
    let mut skipped: BTreeMap<&'static str, usize> = BTreeMap::new();
    for slot in &slots {
        let subject = Subject::real(Ff2Format::new(), slot);
        let built = build(&subject);
        for name in &built.skipped {
            *skipped.entry(*name).or_default() += 1;
        }
        for name in ALWAYS {
            assert!(!built.skipped.contains(&name), "{}: the case {name} is skipped", subject.label);
        }
        spells += usize::from(built.cases.iter().any(|case| case.name == "spell_up" || case.name == "spell_down"));
        for case in &built.cases {
            run(&subject, case);
            cases_run += 1;
            runs += case.runs.len();
        }
    }
    report(&format!(
        "ff2 private exit: {} slots ({spells} with a spell case), {cases_run} cases, {runs} proofs, cases not built: {}",
        slots.len(),
        skipped_line(&skipped),
    ));
}

#[test]
fn real_empty_edits_round_trip_every_slot() {
    let Some(slots) = slots() else { return };
    for slot in &slots {
        let subject = Subject::real(Ff2Format::new(), slot);
        let mut document = subject.open();
        subject.assert_original(&*document, "decode and encode");
        must(document.view(), &format!("{}: the view does not read", subject.label));
        subject.assert_original(&*document, "after a view");
        must(
            document.apply_all(&ChangeSet::default(), CapPolicy::StorageOnly),
            &format!("{}: the empty change set is refused", subject.label),
        );
        subject.assert_original(&*document, "after an empty edit");
        let files = must(document.encode(), &format!("{}: encode failed", subject.label));
        let reopened = must(subject.reopen(&files), &format!("{}: the encoding does not reopen", subject.label));
        subject.assert_original(&*reopened, "decode, encode, decode");
    }
}

#[test]
fn real_level_proofs_fail_without_the_dictionary_copy() {
    let Some(slots) = slots() else { return };
    let mut checked = 0;
    for slot in &slots {
        let subject = Subject::real(Ff2Format::new(), slot);
        let Some(case) = level_without_copy(&subject) else { continue };
        let outcome = catch_unwind(AssertUnwindSafe(|| run(&subject, &case)));
        let payload = must_some(outcome.err(), &format!("{}: the proof passes without the copy", subject.label));
        let message = payload.downcast_ref::<String>().cloned().unwrap_or_default();
        assert!(
            message.contains("unexpected change") && message.contains("abilityDictionary"),
            "{}: the proof fails for another reason",
            subject.label
        );
        checked += 1;
    }
    report(&format!(
        "ff2 private exit: the copy-less Level proof failed on {checked} of {} slots",
        slots.len()
    ));
}

#[test]
fn real_refused_edits_write_nothing() {
    let Some(slots) = slots() else { return };
    for slot in &slots {
        let subject = Subject::real(Ff2Format::new(), slot);
        let held = inventory_ids(&subject);
        let worn = worn_ids(&subject);
        let inventory = |id: u32| RowId::new(FieldId::new(names::INVENTORY), id);
        if let Some(id) = worn.iter().find(|id| held.contains(id) && !is_placeholder::<Ff2Model>(**id)) {
            refuses(&subject, "remove a worn row", &Edit::remove(inventory(*id)), is_read_only);
        }
        if let Some(id) = held.iter().find(|id| is_placeholder::<Ff2Model>(**id)) {
            refuses(&subject, "remove a placeholder row", &Edit::remove(inventory(*id)), is_read_only);
        }
        let mut document = subject.open();
        let view = must(document.view(), &format!("{}: the view does not read", subject.label));
        let party = must_some(view.section(Capability::Party), &format!("{}: no Party section", subject.label));
        let Some([_, Key::Id(character)]) = party.entities.first().map(|entity| entity.id.keys()) else {
            panic!("{}: no character", subject.label)
        };
        let field = |name| common::character_field(*character, name);
        let hand = field(names::EQUIPMENT).with_id(1);
        refuses(
            &subject,
            "set a slot count",
            &Edit::set(hand.clone().with_name(names::COUNT), Value::Int(1)),
            is_read_only,
        );
        // The shield placeholder in a hand is no state the game writes.
        refuses(&subject, "set a hand to 208", &Edit::set(hand, Value::Int(208)), |error| {
            matches!(error, EditError::UnknownId { .. })
        });
        refuses(&subject, "set a derived field", &Edit::set(field(names::MAX_HP), Value::Int(1)), is_read_only);
        let spell = view
            .section(Capability::Magic)
            .into_iter()
            .flat_map(|section| &section.entities)
            .flat_map(|entity| &entity.fields)
            .find(|field| matches!(field.id.keys(), [_, _, Key::Name(names::SPELL), _]));
        if let Some(spell) = spell {
            // One `Set` cannot return the exact inverse of the places a level moves.
            let level = Edit::set(spell.id.clone(), Value::Int(1));
            for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
                let result = subject.open().apply(&level, policy);
                assert!(
                    result.err().is_some_and(|error| is_read_only(&error)),
                    "{}: a raw Level set is not read only",
                    subject.label
                );
            }
        }
    }
}
