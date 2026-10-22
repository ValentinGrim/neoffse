//! Level and Experience move together under `apply_game`
//! (`docs/formats/pixel-remaster.md#level-up-in-ff-i`). Expected totals come
//! from `data/pr/ff1/growth.toml`: level 31 starts at 94025, level 32 at
//! 103811, level 40 at 207299, level 99 at 1471455. A level increase also
//! grows the stats; these tests script every roll to fail and assert the link
//! and the first two edits, and `ff1_levelup.rs` holds the growth values.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{SlotData, character_field, model, standard_model};
use common::rolls::Script;
use neoffse_core::{CapPolicy, Edit, EditError, FieldId, GameChange, Value};
use neoffse_format_pr::ff1::{Ff1Model, names};

fn level(id: u32) -> FieldId {
    character_field(id, names::LEVEL)
}

fn exp(id: u32) -> FieldId {
    character_field(id, names::EXP)
}

fn set(field: FieldId, value: i64) -> Edit {
    Edit::set(field, Value::Int(value))
}

fn int(model: &mut Ff1Model, field: &FieldId) -> i64 {
    match model.get(field).unwrap() {
        Value::Int(value) => value,
        other => panic!("{field}: {other:?}"),
    }
}

/// Applies `edit` under the game interface with every roll failing, checks
/// that the inverse gives the document back byte for byte and returns the
/// change.
fn game(model: &mut Ff1Model, edit: &Edit) -> GameChange {
    let before = model.document().to_json();
    let change = model.apply_game_with(edit, &mut Script::fixed(false, 20)).unwrap();
    change
        .inverse
        .apply_with(CapPolicy::StorageOnly, |step, policy| model.apply(step, policy))
        .unwrap();
    assert_eq!(model.document().to_json(), before, "undo of {edit:?}");
    model.apply_game_with(edit, &mut Script::fixed(false, 20)).unwrap();
    change
}

#[test]
fn a_level_edit_sets_the_first_point_of_the_level() {
    let mut model = standard_model();
    let change = game(&mut model, &set(level(1), 40));
    let edits = change.forward.edits();
    assert_eq!(edits[..2], [set(level(1), 40), set(exp(1), 207_299)]);
    assert!(edits.len() > 2, "the nine levels gained grow the stats");
    assert_eq!(int(&mut model, &level(1)), 40);
    assert_eq!(int(&mut model, &exp(1)), 207_299);
    assert_eq!(int(&mut model, &character_field(1, names::NEXT_LEVEL)), 16_401);
}

#[test]
fn level_bounds() {
    let mut model = standard_model();
    game(&mut model, &set(level(1), 1));
    assert_eq!(int(&mut model, &exp(1)), 0);
    game(&mut model, &set(level(1), 99));
    assert_eq!(int(&mut model, &exp(1)), 1_471_455);
    assert!(matches!(model.get(&character_field(1, names::NEXT_LEVEL)), Err(EditError::UnknownField { .. })));
    let before = model.document().to_json();
    let zero = model.apply_game(&set(level(1), 0));
    assert!(matches!(zero, Err(EditError::OutOfRange { value: 0, min: 1, max: 99, .. })), "{zero:?}");
    let above = model.apply_game(&set(level(1), 100));
    assert!(matches!(above, Err(EditError::AboveCap { cap: 99, value: 100, .. })), "{above:?}");
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn an_exp_edit_sets_the_level_it_reaches() {
    // (experience, level reached): the totals of levels 36, 98 and 99 bound the cases.
    let cases = [(150_000, 36), (0, 1), (1_471_454, 98), (1_471_455, 99), (9_999_999, 99)];
    for (value, reached) in cases {
        let mut model = standard_model();
        let change = game(&mut model, &set(exp(1), value));
        assert_eq!(change.forward.edits()[..2], [set(exp(1), value), set(level(1), reached)], "exp {value}");
        assert_eq!(int(&mut model, &level(1)), reached, "exp {value}");
        assert_eq!(int(&mut model, &exp(1)), value, "exp {value}");
    }
    let mut model = standard_model();
    game(&mut model, &set(exp(1), 150_000));
    assert_eq!(int(&mut model, &character_field(1, names::NEXT_LEVEL)), 12_926);
}

#[test]
fn an_unchanged_linked_field_is_left_out() {
    // Level 31 starts at 94025: the standard slot (exp 100100) is in band but
    // not on the first point, so only a slot at 94025 shows the omission.
    let mut model = standard_model();
    let before = model.document().to_json();
    let change = model.apply_game(&set(exp(1), 100_100)).unwrap();
    assert_eq!(change.forward.edits(), [set(exp(1), 100_100)]);
    assert_eq!(model.document().to_json(), before);
    let change = model.apply_game(&set(level(1), 31)).unwrap();
    assert_eq!(change.forward.edits(), [set(level(1), 31), set(exp(1), 94_025)]);
    let on_the_point = model.document().to_json();
    let change = model.apply_game(&set(level(1), 31)).unwrap();
    assert_eq!(change.forward.edits(), [set(level(1), 31)]);
    assert_eq!(model.document().to_json(), on_the_point);
}

#[test]
fn an_out_of_band_save_is_repaired_by_the_next_edit() {
    let mut model = standard_model();
    let change = model.apply_game(&set(exp(2), 200_200)).unwrap();
    assert_eq!(change.forward.edits()[..2], [set(exp(2), 200_200), set(level(2), 39)]);
    let mut model = standard_model();
    let change = model.apply_game(&set(level(2), 32)).unwrap();
    assert_eq!(change.forward.edits(), [set(level(2), 32), set(exp(2), 103_811)]);
}

#[test]
fn raw_edits_link_nothing() {
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        let mut model = standard_model();
        model.apply(&set(level(1), 40), policy).unwrap();
        assert_eq!(int(&mut model, &exp(1)), 100_100);
        model.apply(&set(level(1), 31), policy).unwrap();
        model.apply(&set(exp(1), 0), policy).unwrap();
        assert_eq!(int(&mut model, &level(1)), 31);
        model.apply(&set(level(1), 0), policy).unwrap();
        assert_eq!(int(&mut model, &level(1)), 0);
    }
}

#[test]
fn a_failed_linked_step_gives_the_document_back() {
    for (omitted, edit, linked) in [("currentExp", set(level(1), 40), exp(1)), ("addtionalLevel", set(exp(1), 150_000), level(1))] {
        let mut data = SlotData::standard();
        data.characters[0].omit = Some(omitted);
        let mut model = model(&data.render());
        let before = model.document().to_json();
        let read = model.get(&linked).unwrap_err();
        let error = model.apply_game(&edit).unwrap_err();
        assert_eq!(std::mem::discriminant(&error), std::mem::discriminant(&read), "{omitted}: {error:?}");
        assert_eq!(model.document().to_json(), before, "{omitted}");
    }
}

#[test]
fn the_spaced_slot_behaves_the_same() {
    let mut data = SlotData::standard();
    data.spaced = true;
    let mut model = model(&data.render());
    model.apply_game_with(&set(level(1), 40), &mut Script::fixed(false, 20)).unwrap();
    assert_eq!(int(&mut model, &exp(1)), 207_299);
    model.apply_game(&set(exp(1), 150_000)).unwrap();
    assert_eq!(int(&mut model, &level(1)), 36);
}
