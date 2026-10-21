//! The cases that disprove the M3 exit proof if it were hollow: a refused
//! edit and a set to the current value leave the file as it was, a spaced
//! text shows the rewrite stay inside the edited chain, a float that `serde_json`
//! prints differently survives, and each container comes back unchanged.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::edit_proof::{How, Proof, Subject, prove};
use common::exit_cases::{Case, build, run};
use common::ff1_save::{SlotData, character_field};
use common::json_diff::{diff, layout_chain};
use common::json_tree::Tree;
use neoffse_core::{Edit, EditError, FieldId, RowId, SaveDocument, Value};
use neoffse_format_pr::container::{self, Container};
use neoffse_format_pr::ff1::names;

const GIL: &str = "userData.owendGil";
const PLAY_TIME: &str = "userData.playTime";
/// A float whose shortest `f64` print differs from this text.
const PLAY_TIME_TEXT: &str = "13621.814333377138";

/// A named edit and the check on the error it must meet.
type Refusal = (&'static str, Edit, fn(&EditError) -> bool);

fn subject() -> Subject {
    Subject::fixture(&SlotData::standard(), Container::game())
}

fn set_gil(number: i64) -> Edit {
    Edit::set(FieldId::new(names::GIL), Value::Int(number))
}

fn empty_proof() -> Proof {
    Proof {
        changes: Vec::new(),
        view: Vec::new(),
        texts: Vec::new(),
    }
}

fn slot(character: u32) -> FieldId {
    character_field(character, names::EQUIPMENT).with_id(1)
}

fn inventory(key: u32) -> RowId {
    RowId::new(FieldId::new(names::INVENTORY), key)
}

/// The decoded text and container of `document`, through the independent codec.
fn decoded(subject: &Subject, document: &dyn SaveDocument) -> container::Decoded {
    let files = document.encode().unwrap();
    container::decode(files.get(&subject.name).unwrap()).unwrap()
}

fn assert_original(subject: &Subject, document: &dyn SaveDocument, what: &str) {
    let after = decoded(subject, document);
    assert!(after.json == subject.json, "{what}: the text changed");
    assert_eq!(after.container, subject.container, "{what}: the container changed");
}

/// The tree after `case`'s edits, applied through `apply_game`.
fn edited(subject: &Subject, case: &Case) -> Tree {
    let mut document = subject.open();
    for edit in &case.edits {
        document.apply_game(edit).unwrap();
    }
    Tree::parse(&decoded(subject, &*document).json)
}

fn find<'a>(cases: &'a [Case], name: &str) -> &'a Case {
    cases.iter().find(|case| case.name == name).unwrap()
}

#[test]
fn a_set_to_the_current_value_changes_nothing() {
    let subject = subject();
    let current = subject.tree.int(GIL).unwrap();
    let mut document = subject.open();
    document.apply_game(&set_gil(current)).unwrap();
    let after = decoded(&subject, &*document);
    assert_eq!(diff(&subject.json, &after.json), Vec::new());
    assert_original(&subject, &*document, "set to the current value");
    for how in [How::Game, How::Raw] {
        prove(&subject, "same value", &[set_gil(current)], how, &empty_proof());
    }
}

#[test]
fn refused_edits_write_nothing() {
    let subject = subject();
    let refusals: [Refusal; 5] = [
        ("level above the cap", Edit::set(character_field(1, names::LEVEL), Value::Int(100)), |error| {
            matches!(error, EditError::AboveCap { .. })
        }),
        ("remove a worn row", Edit::remove(inventory(89)), |error| {
            matches!(error, EditError::ReadOnly { .. })
        }),
        ("set a slot count", Edit::set(slot(1).with_name(names::COUNT), Value::Int(3)), |error| {
            matches!(error, EditError::ReadOnly { .. })
        }),
        ("equip an unknown id", Edit::set(slot(1), Value::Int(9999)), |error| {
            matches!(error, EditError::UnknownId { id: 9999, .. })
        }),
        ("remove a placeholder row", Edit::remove(inventory(62)), |error| {
            matches!(error, EditError::ReadOnly { .. })
        }),
    ];
    let mut shared = subject.open();
    for (what, edit, expected) in refusals {
        let mut fresh = subject.open();
        let Err(error) = fresh.apply_game(&edit) else {
            panic!("{what}: the edit is accepted")
        };
        assert!(expected(&error), "{what}: wrong error: {error}");
        assert_original(&subject, &*fresh, what);
        assert!(shared.apply_game(&edit).is_err(), "{what}: accepted on the shared document");
    }
    assert_original(&subject, &*shared, "five refusals in a row");
}

#[test]
fn the_edit_matrix_on_a_spaced_text() {
    let spaced = subject().respaced(4);
    assert!(spaced.json != spaced.tree.spaced(0), "the text is not spaced");
    let built = build(&spaced);
    assert!(built.skipped.is_empty(), "skipped: {:?}", built.skipped);
    for (name, paths) in [("gil", 2), ("level", 4), ("insert_item_middle", 4), ("remove_item", 4)] {
        let chain = layout_chain(&spaced.tree, &find(&built.cases, name).proof.changes);
        assert_eq!(chain.len(), paths, "{name}: the layout chain");
    }
    for case in &built.cases {
        run(&spaced, case);
    }
}

#[test]
fn a_non_shortest_float_survives_every_edit() {
    let printed = serde_json::to_string(&13_621.814_333_377_138_f64).unwrap();
    assert_ne!(printed, PLAY_TIME_TEXT, "serde_json prints the float as the save holds it");
    let subject = subject();
    assert_eq!(subject.tree.raw_at(PLAY_TIME), Some(PLAY_TIME_TEXT));
    let built = build(&subject);
    for name in ["gil", "level", "insert_item_middle", "equip_new"] {
        let after = edited(&subject, find(&built.cases, name));
        assert_eq!(after.raw_at(PLAY_TIME), Some(PLAY_TIME_TEXT), "{name}: the float changed");
    }
}

#[test]
fn each_container_survives_an_edit() {
    let containers = [
        Container::game(),
        Container { bom: true, trailer: vec![] },
        Container {
            bom: false,
            trailer: b" \r\n\t".to_vec(),
        },
    ];
    for container in containers {
        let subject = Subject::fixture(&SlotData::standard(), container);
        let built = build(&subject);
        run(&subject, find(&built.cases, "gil"));
    }
}
