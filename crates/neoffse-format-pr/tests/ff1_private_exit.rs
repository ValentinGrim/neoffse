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
use common::ff1_real::slots;
use neoffse_core::{Capability, Edit, EditError, FieldId, Key, RowId, SaveDocument, Value};
use neoffse_format_pr::container;
use neoffse_format_pr::ff1::{GAME, names};

const CHARS: &str = "userData.ownedCharacterList.target";
const INV: &str = "userData.normalOwnedItemList.target";
const KEYS: &str = "userData.importantOwendItemList.target";
const SORT_LIST: &str = "userData.normalOwnedItemSortIdList.target";

#[allow(clippy::print_stderr, reason = "the counts of the run are the only output a skipped-or-not test has")]
fn report(line: &str) {
    eprintln!("{line}");
}

/// The root, `userData`, both item list documents and their `target` arrays
/// carry no whitespace of their own.
fn is_compact(subject: &Subject) -> bool {
    ["", "userData", "userData.normalOwnedItemList", INV, "userData.importantOwendItemList", KEYS]
        .iter()
        .all(|path| subject.tree.at(path).is_none_or(|node| node.own_ws().is_empty()))
}

fn skipped_line(skipped: &BTreeMap<&'static str, usize>) -> String {
    if skipped.is_empty() {
        return "none".to_owned();
    }
    skipped.iter().map(|(name, count)| format!("{name} x{count}")).collect::<Vec<_>>().join(", ")
}

#[test]
fn real_slots_prove_every_edit_kind() {
    let Some(slots) = slots() else { return };
    let (mut compact, mut cases_run, mut raw_run, mut sort_lists) = (0, 0, 0, 0);
    let mut skipped: BTreeMap<&'static str, usize> = BTreeMap::new();
    for slot in &slots {
        let subject = Subject::real(slot);
        compact += usize::from(is_compact(&subject));
        sort_lists += usize::from(subject.tree.array_len(SORT_LIST).is_some_and(|len| len > 0));
        let built = build(&subject);
        for name in &built.skipped {
            *skipped.entry(*name).or_default() += 1;
        }
        for name in ALWAYS {
            assert!(!built.skipped.contains(&name), "{}: the case {name} is skipped", subject.label);
        }
        for case in &built.cases {
            run(&subject, case);
            cases_run += 1;
            raw_run += usize::from(case.raw);
        }
    }
    report(&format!(
        "ff1 private exit: {} slots ({compact} compact), {cases_run} cases run ({raw_run} also through apply), skipped: {}, non-empty sort lists: {sort_lists}",
        slots.len(),
        skipped_line(&skipped),
    ));
}

/// The decoded text and container of `document` equal the subject's.
fn assert_original(subject: &Subject, document: &dyn SaveDocument, what: &str) {
    let files = document.encode().unwrap();
    let after = container::decode(files.get(&subject.name).unwrap()).unwrap();
    assert!(after.json == subject.json, "{} {what}: the text changed", subject.label);
    assert!(after.container == subject.container, "{} {what}: the container changed", subject.label);
}

fn is_placeholder(id: u32) -> bool {
    neoffse_data::tables_for(GAME)
        .and_then(|tables| tables.content(id))
        .is_some_and(|entry| entry.placeholder)
}

/// Every content id that a character wears, in document order.
fn worn_ids(subject: &Subject) -> Vec<u32> {
    let mut ids = Vec::new();
    for index in 0..subject.tree.array_len(CHARS).unwrap_or(0) {
        let list = format!("{CHARS}[{index}].equipmentList.values");
        for slot in 0..subject.tree.array_len(&list).unwrap_or(0) {
            let id = subject.tree.int(&format!("{list}[{slot}].contentId"));
            ids.extend(id.and_then(|id| u32::try_from(id).ok()));
        }
    }
    ids
}

/// The inventory ids in document order.
fn inventory_ids(subject: &Subject) -> Vec<u32> {
    (0..subject.tree.array_len(INV).unwrap_or(0))
        .filter_map(|index| subject.tree.int(&format!("{INV}[{index}].contentId")))
        .filter_map(|id| u32::try_from(id).ok())
        .collect()
}

fn refuses(subject: &Subject, what: &str, edit: &Edit) {
    let mut document = subject.open();
    let result = document.apply_game(edit);
    assert!(
        matches!(result, Err(EditError::ReadOnly { .. })),
        "{} {what}: not refused as read only",
        subject.label
    );
    assert_original(subject, &*document, what);
}

#[test]
fn real_refused_edits_write_nothing() {
    let Some(slots) = slots() else { return };
    for slot in &slots {
        let subject = Subject::real(slot);
        let held = inventory_ids(&subject);
        let worn = worn_ids(&subject);
        let worn_row = worn.iter().find(|id| held.contains(id) && !is_placeholder(**id));
        let placeholder_row = held.iter().find(|id| is_placeholder(**id));
        if let Some(id) = worn_row {
            refuses(&subject, "remove a worn row", &Edit::remove(RowId::new(FieldId::new(names::INVENTORY), *id)));
        }
        if let Some(id) = placeholder_row {
            refuses(
                &subject,
                "remove a placeholder row",
                &Edit::remove(RowId::new(FieldId::new(names::INVENTORY), *id)),
            );
        }
        let mut document = subject.open();
        let view = document.view().unwrap();
        let party = view.section(Capability::Party).unwrap();
        let Some([_, Key::Id(character)]) = party.entities.first().map(|entity| entity.id.keys()) else {
            panic!("{}: no character", subject.label)
        };
        let count = common::ff1_save::character_field(*character, names::EQUIPMENT)
            .with_id(1)
            .with_name(names::COUNT);
        refuses(&subject, "set a slot count", &Edit::set(count, Value::Int(1)));
    }
}
