//! The M3 exit proof on every private FF I slot: the edit matrix, then the
//! refusals that must write nothing. The saves are read-only input and the
//! tests write nothing to disk. A failure names the slot label, the case, the
//! interface and a path or a check, never a value, a length or a name: the
//! files hold personal data. A slot that is not compact adds the expected
//! `Layout` chain to every expected list and is counted, not failed.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;

use common::edit_proof::Subject;
use common::exit_cases::{ALWAYS, build, run};
use common::exit_facts::{CHARS, INV, KEYS};
use common::ff1_real::slots;
use common::private::{inventory_ids, is_placeholder, is_read_only, refuses, skipped_line, tables, worn_ids};
use common::real_slots::{must, must_some};
use neoffse_core::{Capability, Edit, FieldId, Key, RowId, Value};
use neoffse_format_pr::ff1::{Ff1Format, Ff1Model, names};
use neoffse_testkit::report;

const SORT_LIST: &str = "userData.normalOwnedItemSortIdList.target";

/// The root, `userData`, both item list documents and their `target` arrays
/// carry no whitespace of their own.
fn is_compact(subject: &Subject) -> bool {
    ["", "userData", "userData.normalOwnedItemList", INV, "userData.importantOwendItemList", KEYS]
        .iter()
        .all(|path| subject.tree.at(path).is_none_or(|node| node.own_ws().is_empty()))
}

/// The stored level of the first character, the one the matrix edits.
fn first_level(subject: &Subject) -> Option<i64> {
    subject.tree.int(&format!("{CHARS}[0].parameter.addtionalLevel"))
}

/// The game's level cap, `MAX_LV`.
fn max_level() -> i64 {
    let tables = tables::<Ff1Model>();
    must_some(tables.bound("MAX_LV"), "no MAX_LV in the FF I tables")
}

#[test]
fn real_slots_prove_every_edit_kind() {
    let Some(slots) = slots() else { return };
    let (mut compact, mut cases_run, mut raw_run, mut sort_lists) = (0, 0, 0, 0);
    let mut skipped: BTreeMap<&'static str, usize> = BTreeMap::new();
    for slot in &slots {
        let subject = Subject::real(Ff1Format::new(), slot);
        compact += usize::from(is_compact(&subject));
        sort_lists += usize::from(subject.tree.array_len(SORT_LIST).is_some_and(|len| len > 0));
        let built = build(&subject);
        for name in &built.skipped {
            *skipped.entry(*name).or_default() += 1;
        }
        let at_cap = first_level(&subject).is_some_and(|level| level >= max_level());
        for name in ALWAYS {
            if name == "level_up" && at_cap {
                continue;
            }
            assert!(!built.skipped.contains(&name), "{}: the case {name} is skipped", subject.label);
        }
        for case in &built.cases {
            run(&subject, case);
            cases_run += 1;
            raw_run += usize::from(case.raw);
        }
    }
    report(&format!(
        "ff1 private exit: {} slots ({compact} compact), {cases_run} cases run ({raw_run} also through apply), cases not built: {}, non-empty sort lists: {sort_lists}",
        slots.len(),
        skipped_line(&skipped),
    ));
}

#[test]
fn real_refused_edits_write_nothing() {
    let Some(slots) = slots() else { return };
    for slot in &slots {
        let subject = Subject::real(Ff1Format::new(), slot);
        let held = inventory_ids(&subject);
        let worn = worn_ids(&subject);
        let worn_row = worn.iter().find(|id| held.contains(id) && !is_placeholder::<Ff1Model>(**id));
        let placeholder_row = held.iter().find(|id| is_placeholder::<Ff1Model>(**id));
        if let Some(id) = worn_row {
            refuses(
                &subject,
                "remove a worn row",
                &Edit::remove(RowId::new(FieldId::new(names::INVENTORY), *id)),
                is_read_only,
            );
        }
        if let Some(id) = placeholder_row {
            refuses(
                &subject,
                "remove a placeholder row",
                &Edit::remove(RowId::new(FieldId::new(names::INVENTORY), *id)),
                is_read_only,
            );
        }
        let mut document = subject.open();
        let view = must(document.view(), &format!("{}: the view does not read", subject.label));
        let party = must_some(view.section(Capability::Party), &format!("{}: no Party section", subject.label));
        let Some([_, Key::Id(character)]) = party.entities.first().map(|entity| entity.id.keys()) else {
            panic!("{}: no character", subject.label)
        };
        let count = common::character_field(*character, names::EQUIPMENT).with_id(1).with_name(names::COUNT);
        refuses(&subject, "set a slot count", &Edit::set(count, Value::Int(1)), is_read_only);
    }
}
