//! The spell Level edit fails whole and undoes exactly: a failed step leaves
//! the bytes and the history alone, a missing dictionary copy writes nothing,
//! and an undo or a redo restores each copy's exact bytes, a mismatched
//! pre-state included (`docs/formats/pixel-remaster.md#row-and-value-in-ff-ii`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_path;
use common::ff2_save::{SlotData, model, standard_model};
use common::ff2_spell::{History, level_edit};
use neoffse_core::{CapPolicy, EditError, FieldId, SaveFormat, Value};
use neoffse_format_pr::container::{Container, decode};
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Format, Ff2Model, names};

use common::ff1_format::{one_file, sav, slot_name};

fn int(model: &mut Ff2Model, field: &FieldId) -> i64 {
    match model.get(field).unwrap() {
        Value::Int(value) => value,
        other => panic!("{field}: {other:?}"),
    }
}

/// A spaced slot whose copy of chain 0 stores `skillLevel` -5: the plan
/// passes, and step 6 fails on the storage bound after five steps wrote.
fn broken_copy() -> String {
    let mut slot = SlotData::standard();
    slot.spaced = true;
    slot.characters[0].spells[0].dict_value = -5;
    slot.render()
}

#[test]
fn a_failed_step_leaves_the_bytes() {
    let text = broken_copy();
    let mut model = model(&text);
    let error = model.apply_game(&level_edit(1, 0, 6)).unwrap_err();
    assert!(matches!(error, EditError::OutOfRange { value: -5, min: 0, .. }), "{error:?}");
    assert_eq!(model.document().to_json(), text);
    // The failed call leaves the stored value readable and the Level unchanged.
    assert_eq!(int(&mut model, &names::place(1, names::DICTIONARY, 0, names::PROFICIENCY)), -5);
    assert_eq!(model.get(&names::spell_field(1, 0)).unwrap(), Value::Int(5));
}

#[test]
fn the_rollback_alone_would_leave_the_spaced_text_rewritten() {
    // The plan of a healthy twin, replayed by hand on the broken slot: the
    // rollback restores the values, and only the snapshot of `apply_game`
    // restores the layout.
    let mut healthy = SlotData::standard();
    healthy.spaced = true;
    let mut twin = model(&healthy.render());
    let forward = twin.apply_game(&level_edit(1, 0, 6)).unwrap().forward;
    let text = broken_copy();
    let mut raw = model(&text);
    let outcome = forward.apply_with(CapPolicy::Enforce, |edit, policy| raw.apply(edit, policy));
    assert!(matches!(outcome, Err(EditError::OutOfRange { value: -5, .. })));
    assert_ne!(raw.document().to_json(), text);
}

#[test]
fn a_failed_edit_leaves_no_history_entry() {
    let text = broken_copy();
    let mut history = History::open(&text);
    let before = history.text();
    let error = history.edit(&level_edit(1, 0, 6)).unwrap_err();
    assert!(matches!(error, EditError::OutOfRange { .. }), "{error:?}");
    assert!(history.undo.is_empty() && history.redo.is_empty());
    assert_eq!(history.text(), before);
    assert!(!history.undo());
    // A later edit on a healthy chain still works and records one entry.
    history.edit(&level_edit(1, 1, 3)).unwrap();
    assert_eq!(history.undo.len(), 1);
}

#[test]
fn a_dictionary_without_key_3_fails_before_any_write() {
    let mut other_key = Document::parse(&SlotData::standard().render()).unwrap();
    let keys = character_path(
        0,
        &[
            Segment::Doc,
            Segment::Key("abilityDictionary"),
            Segment::Doc,
            Segment::Key("keys"),
            Segment::Index(0),
        ],
    );
    other_key.set(&keys, &Scalar::Int(4)).unwrap();
    let mut no_dictionary = SlotData::standard();
    no_dictionary.characters[0].omit = Some("abilityDictionary");
    for (label, mut model) in [
        ("another key", Ff2Model::new(other_key, common::ff2_save::tables())),
        ("no dictionary", model(&no_dictionary.render())),
    ] {
        let before = model.document().to_json();
        for level in [6, 5] {
            let error = model.apply_game(&level_edit(1, 0, level)).unwrap_err();
            assert!(matches!(error, EditError::UnknownField { .. }), "{label}: {error:?}");
            assert_eq!(model.document().to_json(), before, "{label}");
        }
    }
    let mut broken = SlotData::standard();
    broken.characters[0].omit = Some("abilityDictionary");
    let mut history = History::open(&broken.render());
    assert!(history.edit(&level_edit(1, 0, 6)).is_err());
    assert_eq!(history.undo.len(), 0);
}

#[test]
fn an_undo_restores_the_mismatched_pre_state() {
    let mut history = History::open(&SlotData::standard().render());
    let original = history.text();
    // Character 2 holds chain 5 at row level 1 with 456 (list) and 400 (copy).
    for level in [1, 4, 16] {
        history.edit(&level_edit(2, 5, level)).unwrap();
        let edited = history.text();
        assert_ne!(edited, original, "level {level}");
        assert!(history.undo());
        assert_eq!(history.text(), original, "level {level}: undo");
        assert!(history.undo.is_empty(), "level {level}: the session reads clean");
        assert!(history.redo());
        assert_eq!(history.text(), edited, "level {level}: redo");
        assert!(history.undo());
    }
    // The raw values of the pair are back.
    let mut after = Ff2Model::new(Document::parse(&history.text()).unwrap(), common::ff2_save::tables());
    assert_eq!(int(&mut after, &names::place(2, names::ABILITY, 3, names::PROFICIENCY)), 456);
    assert_eq!(int(&mut after, &names::place(2, names::DICTIONARY, 2, names::PROFICIENCY)), 400);
}

#[test]
fn several_edits_undo_and_redo_in_order() {
    let mut history = History::open(&SlotData::standard().render());
    let original = history.text();
    let edits = [level_edit(1, 0, 9), level_edit(1, 0, 2), level_edit(2, 5, 1), level_edit(4, 15, 3)];
    let mut states = vec![original.clone()];
    for edit in &edits {
        history.edit(edit).unwrap();
        states.push(history.text());
    }
    for state in states.iter().rev().skip(1) {
        assert!(history.undo());
        assert_eq!(&history.text(), state);
    }
    assert!(!history.undo());
    for state in states.iter().skip(1) {
        assert!(history.redo());
        assert_eq!(&history.text(), state);
    }
    assert!(!history.redo());
}

#[test]
fn the_trait_carries_the_level_edit_to_the_file() {
    let text = SlotData::standard().render();
    let mut history = History::open(&text);
    let edit = level_edit(1, 0, 6);
    history.edit(&edit).unwrap();
    // The change of the boxed document equals the change of the model.
    let want = standard_model().apply_game(&edit).unwrap();
    assert_eq!(history.undo[0], want);
    // Encode, then decode with the container layer: the document reloads to the same text.
    let files = history.document.encode().unwrap();
    history.document.verify(&files).unwrap();
    let decoded = decode(files.get(slot_name(1)).unwrap()).unwrap();
    assert_eq!(Document::parse(&decoded.json).unwrap().to_json(), decoded.json);
    // A fresh open of the encoded bytes reads the new level and undoes to the old file.
    let format: &dyn SaveFormat = &Ff2Format::new();
    let mut reopened = format.open(&one_file(slot_name(1), sav(&decoded.json, &Container::game()))).unwrap();
    let again = reopened.apply_game(&level_edit(1, 0, 5)).unwrap();
    assert_eq!(again.forward.len(), 8);
    let restored = decode(reopened.encode().unwrap().get(slot_name(1)).unwrap()).unwrap().json;
    assert_eq!(restored, text);
}
