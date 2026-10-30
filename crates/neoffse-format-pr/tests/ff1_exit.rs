//! The M3 exit criterion on the synthetic FF I slot: each edit kind, through
//! `Box<dyn SaveDocument>`, changes the file in the edited tokens and nowhere
//! else. Anchors state their expected change lists by hand.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::edit_proof::{How, Proof, Subject, ViewChange, prove};
use common::exit_cases::{ALWAYS, FIXTURE_SKIPS, NEW_NAME, build, run};
use common::ff1_save::{INVENTORY, KEY_ITEMS, SlotData, character_field};
use common::json_diff::{Change, added, removed, value};
use neoffse_core::{Edit, FieldId, RowId, TableKind, Value};
use neoffse_format_pr::container::Container;
use neoffse_format_pr::ff1::{GAME, names};

const GIL: &str = "userData.owendGil";
const INV: &str = "userData.normalOwnedItemList.target";
const KEYS: &str = "userData.importantOwendItemList.target";

fn subject() -> Subject {
    Subject::fixture(&SlotData::standard(), Container::game())
}

fn texts(pairs: &[(&str, String)]) -> Vec<(String, String)> {
    pairs.iter().map(|(path, text)| ((*path).to_owned(), text.clone())).collect()
}

fn proof(changes: Vec<Change>, view: Vec<ViewChange>, pairs: &[(&str, String)]) -> Proof {
    Proof {
        changes,
        view,
        texts: texts(pairs),
    }
}

fn game(case: &str, edit: Edit, proof: &Proof) {
    prove(&subject(), case, &[edit], How::Game, proof);
}

fn gil() -> FieldId {
    FieldId::new(names::GIL)
}

fn set_gil(number: i64) -> Edit {
    Edit::set(gil(), Value::Int(number))
}

fn gil_proof(number: i64) -> Proof {
    proof(vec![value(GIL)], vec![ViewChange::Set(gil(), Value::Int(number))], &[(GIL, number.to_string())])
}

fn inventory(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

fn slot(character: u32) -> FieldId {
    character_field(character, names::EQUIPMENT).with_id(1)
}

/// The first non-placeholder id of `kind` that `owned` does not list.
fn first_free(kind: TableKind, owned: &[u32]) -> u32 {
    let tables = neoffse_data::tables_for(GAME).unwrap();
    tables
        .table(kind)
        .unwrap()
        .iter()
        .find(|entry| !entry.placeholder && !owned.contains(&entry.id))
        .unwrap()
        .id
}

fn free_item() -> u32 {
    first_free(TableKind::Item, &INVENTORY.map(|(id, _)| id))
}

fn row_text(id: u32, count: u32) -> String {
    format!("{{\"contentId\":{id},\"count\":{count}}}")
}

#[test]
fn gil_changes_only_its_token() {
    game("gil", set_gil(200_000), &gil_proof(200_000));
}

#[test]
fn level_changes_one_token_five_documents_deep() {
    let path = "userData.ownedCharacterList.target[0].parameter.addtionalLevel";
    let level = character_field(1, names::LEVEL);
    let expected = proof(
        vec![value(path)],
        vec![ViewChange::Set(level.clone(), Value::Int(40))],
        &[(path, "40".to_owned())],
    );
    prove(&subject(), "level", &[Edit::set(level, Value::Int(40))], How::Raw, &expected);
}

#[test]
fn a_name_with_a_quote_and_a_backslash_survives() {
    let path = "userData.ownedCharacterList.target[3].name";
    let name = character_field(4, names::NAME);
    let view = vec![ViewChange::Set(name.clone(), Value::Text(NEW_NAME.to_owned()))];
    let expected = proof(vec![value(path)], view, &[(path, r#""Nö\"\\""#.to_owned())]);
    game("name", Edit::set(name, Value::Text(NEW_NAME.to_owned())), &expected);
}

#[test]
fn a_charge_edit_lands_on_the_right_dictionary_entry() {
    let path = "userData.ownedCharacterList.target[1].parameter.currentMpCountList.values[2]";
    let charges = character_field(2, names::CHARGES).with_id(3);
    let expected = proof(
        vec![value(path)],
        vec![ViewChange::Set(charges.clone(), Value::Int(9))],
        &[(path, "9".to_owned())],
    );
    game("charges", Edit::set(charges, Value::Int(9)), &expected);
}

fn insert_proof(row: &RowId, array: &str, index: usize, count: u32) -> Proof {
    let path = format!("{array}[{index}]");
    let view = vec![ViewChange::Insert {
        row: row.clone(),
        position: index,
        count: i64::from(count),
    }];
    proof(vec![added(&path)], view, &[(&path, row_text(row.key(), count))])
}

#[test]
fn insert_in_the_middle_adds_one_row() {
    let row = inventory(free_item());
    let edit = Edit::insert(row.clone(), Some(14), vec![(names::COUNT, Value::Int(3))]);
    game("insert middle", edit, &insert_proof(&row, INV, 14, 3));
}

#[test]
fn insert_at_the_end_adds_one_row() {
    let row = inventory(free_item());
    let edit = Edit::insert(row.clone(), None, vec![(names::COUNT, Value::Int(2))]);
    game("insert end", edit, &insert_proof(&row, INV, 29, 2));
}

#[test]
fn insert_into_key_items_adds_one_row() {
    let row = RowId::new(FieldId::new(names::KEY_ITEMS), first_free(TableKind::KeyItem, &KEY_ITEMS));
    let edit = Edit::insert(row.clone(), None, vec![(names::COUNT, Value::Int(1))]);
    game("insert key item", edit, &insert_proof(&row, KEYS, 9, 1));
}

#[test]
fn remove_drops_one_row() {
    let row = inventory(2);
    let expected = proof(
        vec![removed("userData.normalOwnedItemList.target[5]")],
        vec![ViewChange::Remove(row.clone())],
        &[],
    );
    game("remove", Edit::remove(row.clone()), &expected);
    let change = subject().open().apply_game(&Edit::remove(row)).unwrap();
    let [Edit::Insert { position, .. }] = change.inverse.edits() else {
        panic!("the inverse of a remove is one insert")
    };
    assert_eq!(*position, Some(5));
}

#[test]
fn equip_new_adds_a_row_and_changes_one_slot() {
    let id = "userData.ownedCharacterList.target[0].equipmentList.values[0].contentId";
    let row = inventory(104);
    let view = vec![
        ViewChange::Set(slot(1), Value::Int(104)),
        ViewChange::Insert {
            row: row.clone(),
            position: 29,
            count: 1,
        },
    ];
    let expected = proof(
        vec![value(id), added("userData.normalOwnedItemList.target[29]")],
        view,
        &[(id, "104".to_owned()), ("userData.normalOwnedItemList.target[29]", row_text(104, 1))],
    );
    game("equip new", Edit::set(slot(1), Value::Int(104)), &expected);
}

#[test]
fn equip_held_changes_only_the_id() {
    let id = "userData.ownedCharacterList.target[0].equipmentList.values[0].contentId";
    let expected = proof(vec![value(id)], vec![ViewChange::Set(slot(1), Value::Int(100))], &[(id, "100".to_owned())]);
    game("equip held", Edit::set(slot(1), Value::Int(100)), &expected);
}

#[test]
fn equip_a_shared_item_raises_the_count() {
    let id = "userData.ownedCharacterList.target[1].equipmentList.values[0].contentId";
    let count = "userData.ownedCharacterList.target[1].equipmentList.values[0].count";
    let row = "userData.normalOwnedItemList.target[11].count";
    let view = vec![
        ViewChange::Set(slot(2), Value::Int(89)),
        ViewChange::Set(inventory(89).field(names::COUNT), Value::Int(2)),
        ViewChange::Set(slot(2).with_name(names::COUNT), Value::Int(2)),
    ];
    let texts = [(id, "89".to_owned()), (row, "2".to_owned()), (count, "2".to_owned())];
    let expected = proof(vec![value(id), value(row), value(count)], view, &texts);
    game("equip shared", Edit::set(slot(2), Value::Int(89)), &expected);
}

#[test]
fn unequip_copies_the_placeholder_count() {
    let id = "userData.ownedCharacterList.target[0].equipmentList.values[0].contentId";
    let count = "userData.ownedCharacterList.target[0].equipmentList.values[0].count";
    let view = vec![
        ViewChange::Set(slot(1), Value::Int(62)),
        ViewChange::Set(slot(1).with_name(names::COUNT), Value::Int(5)),
    ];
    let expected = proof(vec![value(id), value(count)], view, &[(id, "62".to_owned()), (count, "5".to_owned())]);
    game("unequip", Edit::set(slot(1), Value::Int(62)), &expected);
}

#[test]
fn every_generated_case_proves_on_the_fixture() {
    let subject = subject();
    let built = build(&subject);
    assert!(built.skipped == FIXTURE_SKIPS, "cases not built: {:?}", built.skipped);
    assert!(ALWAYS.iter().all(|name| built.cases.iter().any(|case| case.name == *name)));
    for case in &built.cases {
        run(&subject, case);
    }
}

#[test]
fn compose_edits_in_two_arrays_at_once() {
    let subject = subject();
    let built = build(&subject);
    let case = built.cases.iter().find(|case| case.name == "compose").unwrap();
    assert_eq!(case.edits.len(), 4);
    run(&subject, case);
}

#[test]
fn the_fixture_matches_the_anchors() {
    let tree = &subject().tree;
    assert_eq!(tree.find_row(INV, "contentId", 89), Some(11));
    for (index, id) in [(26, 100), (27, 103), (28, 165)] {
        assert_eq!(tree.find_row(INV, "contentId", id), Some(index));
    }
    assert_eq!(tree.find_row(INV, "contentId", 2), Some(5));
    let slot = "userData.ownedCharacterList.target[0].equipmentList.values[0]";
    assert_eq!(
        (tree.int(&format!("{slot}.contentId")), tree.int(&format!("{slot}.count"))),
        (Some(89), Some(1))
    );
    assert_eq!(tree.array_len(INV), Some(29));
    assert_eq!(tree.array_len(KEYS), Some(9));
}

#[test]
#[should_panic(expected = "unexpected change")]
fn prove_fails_on_a_stray_change() {
    let declared = Proof {
        changes: Vec::new(),
        ..gil_proof(31)
    };
    game("stray", set_gil(31), &declared);
}

#[test]
#[should_panic(expected = "missing change")]
fn prove_fails_on_a_missing_change() {
    let mut declared = gil_proof(31);
    declared.changes.push(value("userData.playTime"));
    game("missing", set_gil(31), &declared);
}

#[test]
#[should_panic(expected = "unexpected change")]
fn prove_fails_on_two_edits_declared_as_one() {
    let level = character_field(1, names::LEVEL);
    let edits = [set_gil(31), Edit::set(level, Value::Int(40))];
    prove(&subject(), "two edits", &edits, How::Game, &gil_proof(31));
}

#[test]
#[should_panic(expected = "view differs")]
fn prove_fails_on_a_wrong_view_expectation() {
    let declared = Proof {
        view: vec![ViewChange::Set(gil(), Value::Int(32))],
        ..gil_proof(31)
    };
    game("wrong view", set_gil(31), &declared);
}

#[test]
#[should_panic(expected = "wrong text")]
fn prove_fails_on_a_wrong_text() {
    let declared = Proof {
        texts: texts(&[(GIL, "31.0".to_owned())]),
        ..gil_proof(31)
    };
    game("wrong text", set_gil(31), &declared);
}
