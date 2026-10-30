//! `Session::changes` on the standard slot: jobs, inventory rows, view order.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{gil, open_session, set};
use neoffse::core::{Capability, Edit, FieldId, Game, RowId, SaveView, Value};
use neoffse::data::{Language, tables_for};
use neoffse::pr::ff1::names;
use neoffse::{Change, ChangeKind};

fn aria(name: &'static str) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(1).with_name(name)
}

fn row(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

fn field(id: FieldId, label: &str, before: &str, after: &str) -> ChangeKind {
    ChangeKind::Field {
        field: id,
        label: label.to_owned(),
        before: before.to_owned(),
        after: after.to_owned(),
    }
}

fn int(view: &SaveView, id: &FieldId) -> i64 {
    let (_, found) = view.find(id).expect("the field is in the view");
    let Value::Int(number) = found.value else { panic!("{id} is an integer") };
    number
}

#[test]
fn the_standard_slot_holds_the_documented_values() {
    let (_dir, mut session) = open_session();
    let view = session.view().unwrap();
    assert_eq!(int(&view, &gil()), 123_456);
    for (name, expected) in [
        (names::LEVEL, 31),
        (names::EXP, 100_100),
        (names::ACCURACY_GAIN, 143),
        (names::MAGIC_EVASION_GAIN, 133),
        (names::JOB, 7),
    ] {
        assert_eq!(int(&view, &aria(name)), expected, "{name}");
    }
    assert_eq!(int(&view, &row(2).field(names::COUNT)), 7);
    assert_eq!(int(&view, &row(10).field(names::COUNT)), 3);
    assert!(view.find(&row(4).field(names::COUNT)).is_none(), "content 4 is in no row");
    assert_eq!(tables_for(Game::Ff1Pr).unwrap().exp_to_reach(34), Some(125_357));
}

#[test]
fn a_job_edit_names_the_jobs() {
    let (_dir, mut session) = open_session();
    session.edit(&set(&aria(names::JOB), 8)).unwrap();
    let changes = session.changes(Language::En).unwrap();
    let party = FieldId::new(names::CHARACTER).with_id(1);
    assert_eq!(
        changes,
        [Change::new(
            Capability::Party,
            party.clone(),
            "Aria",
            field(aria(names::JOB), "Job", "Knight", "Ninja")
        )]
    );
    let french = session.changes(Language::Fr).unwrap();
    assert_eq!(
        french,
        [Change::new(
            Capability::Party,
            party,
            "Aria",
            field(aria(names::JOB), "Job", "Paladin", "Ninja")
        )]
    );
}

#[test]
fn a_count_edit_shows_the_item() {
    let (_dir, mut session) = open_session();
    let count = row(2).field(names::COUNT);
    session.edit(&set(&count, 9)).unwrap();
    assert_eq!(
        session.changes(Language::En).unwrap(),
        [Change::new(
            Capability::Inventory,
            row(2).id().clone(),
            "Potion",
            field(count, "Count", "7", "9")
        )]
    );
}

#[test]
fn a_row_added_and_a_row_removed() {
    let (_dir, mut session) = open_session();
    session.edit(&Edit::insert(row(4), None, Vec::new())).unwrap();
    session.edit(&Edit::remove(row(10))).unwrap();
    assert_eq!(
        session.changes(Language::En).unwrap(),
        [
            Change::new(Capability::Inventory, row(4).id().clone(), "X-Potion", ChangeKind::RowAdded),
            Change::new(Capability::Inventory, row(10).id().clone(), "Phoenix Down", ChangeKind::RowRemoved),
        ]
    );
}

#[test]
fn a_removed_row_added_again_shows_its_count() {
    let (_dir, mut session) = open_session();
    session.edit(&Edit::remove(row(10))).unwrap();
    session.edit(&Edit::insert(row(10), None, vec![(names::COUNT, Value::Int(1))])).unwrap();
    assert_eq!(
        session.changes(Language::En).unwrap(),
        [Change::new(
            Capability::Inventory,
            row(10).id().clone(),
            "Phoenix Down",
            field(row(10).field(names::COUNT), "Count", "3", "1")
        )]
    );
}

#[test]
fn the_order_follows_the_view() {
    let (_dir, mut session) = open_session();
    session.edit(&set(&gil(), 123_457)).unwrap();
    session.edit(&set(&row(2).field(names::COUNT), 9)).unwrap();
    session.edit(&Edit::set(aria(names::NAME), Value::Text("Zed".to_owned()))).unwrap();

    let changes = session.changes(Language::En).unwrap();
    let party = FieldId::new(names::CHARACTER).with_id(1);
    assert_eq!(
        changes,
        [
            Change::new(Capability::Party, party, "Zed", field(aria(names::NAME), "Name", "Aria", "Zed")),
            Change::new(Capability::Inventory, FieldId::new(names::GIL), "Gil", field(gil(), "Gil", "123456", "123457")),
            Change::new(
                Capability::Inventory,
                row(2).id().clone(),
                "Potion",
                field(row(2).field(names::COUNT), "Count", "7", "9")
            ),
        ]
    );
}
