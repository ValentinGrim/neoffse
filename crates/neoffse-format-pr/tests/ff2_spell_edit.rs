//! The spell Level edit of the FF II game interface on the synthetic slot:
//! the eight places one edit writes, the three states of the order lists, the
//! repair of a mismatched pair and the refusals
//! (`docs/formats/pixel-remaster.md#row-and-value-in-ff-ii`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fmt::Write;

use common::character_path;
use common::ff2_save::{SlotData, Spell, content_of, model, standard_model, tables};
use common::ff2_spell::{History, Where, level_edit, replay};
use common::json_diff::{Kind, assert_changes, diff};
use common::slot_model::int;
use neoffse_core::{Edit, EditError, FieldId, Game, Value};
use neoffse_data::{GameTables, Sources};
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Model, names};

const PARTS: [&str; 3] = ["abilityId", "contentId", "skillLevel"];

/// The three stored values of the entry `position` of `group`.
fn entry(model: &mut Ff2Model, id: u32, group: &'static str, position: u32) -> [i64; 3] {
    [names::ABILITY_ID, names::CONTENT_ID, names::PROFICIENCY].map(|part| int(model, &names::place(id, group, position, part)))
}

/// The row, content id and total of `level` in `chain`.
fn row(chain: u32, level: u32) -> [i64; 3] {
    let ability = tables().spell_ability(chain, level).unwrap();
    [
        i64::from(ability),
        i64::from(content_of(ability)),
        i64::from(tables().skill_total(level).unwrap()),
    ]
}

fn order_at(model: &mut Ff2Model, id: u32, order: &'static str, position: u32) -> i64 {
    int(model, &names::order_place(id, order, position))
}

struct Case {
    id: u32,
    chain: u32,
    from: u32,
    to: u32,
    at: Where,
}

/// Character 1 holds 711, then chains 0, 1 and 2 at levels 5, 1 and 16. The
/// addition order lists 711 first; the sort order lists the spells reversed.
fn cases() -> Vec<Case> {
    let place = |list, copy, addition, sort| Where {
        character: 0,
        list,
        copy,
        addition: Some(addition),
        sort: Some(sort),
    };
    vec![
        Case {
            id: 1,
            chain: 0,
            from: 5,
            to: 6,
            at: place(1, 0, 1, 2),
        },
        Case {
            id: 1,
            chain: 0,
            from: 5,
            to: 4,
            at: place(1, 0, 1, 2),
        },
        Case {
            id: 1,
            chain: 0,
            from: 5,
            to: 16,
            at: place(1, 0, 1, 2),
        },
        Case {
            id: 1,
            chain: 0,
            from: 5,
            to: 1,
            at: place(1, 0, 1, 2),
        },
        Case {
            id: 1,
            chain: 1,
            from: 1,
            to: 2,
            at: place(2, 1, 2, 1),
        },
        Case {
            id: 1,
            chain: 1,
            from: 1,
            to: 16,
            at: place(2, 1, 2, 1),
        },
        Case {
            id: 1,
            chain: 2,
            from: 16,
            to: 15,
            at: place(3, 2, 3, 0),
        },
        Case {
            id: 1,
            chain: 2,
            from: 16,
            to: 1,
            at: place(3, 2, 3, 0),
        },
    ]
}

#[test]
fn a_level_edit_writes_the_eight_places_and_nothing_else() {
    for case in cases() {
        let label = format!("chain {} {}->{}", case.chain, case.from, case.to);
        let mut model = standard_model();
        let before = model.document().to_json();
        let change = model.apply_game(&level_edit(case.id, case.chain, i64::from(case.to))).unwrap();
        assert_eq!(change.forward.len(), 8, "{label}");
        assert!(
            change.forward.edits().iter().all(|edit| matches!(edit, Edit::Set { .. })),
            "{label}: plain edits"
        );
        assert_eq!(change.inverse.len(), 8, "{label}");
        assert_eq!(
            model.get(&names::spell_field(case.id, case.chain)).unwrap(),
            Value::Int(i64::from(case.to)),
            "{label}"
        );
        assert_changes(&label, &diff(&before, &model.document().to_json()), &case.at.changes(&PARTS));
        // The inverse restores every place to its exact old bytes.
        replay(&mut model, &change.inverse).unwrap();
        assert_eq!(model.document().to_json(), before, "{label}: undo");
    }
}

#[test]
fn both_copies_hold_the_new_pair() {
    for case in cases() {
        let mut model = standard_model();
        model.apply_game(&level_edit(case.id, case.chain, i64::from(case.to))).unwrap();
        let wanted = row(case.chain, case.to);
        let list = u32::try_from(case.at.list).unwrap();
        let copy = u32::try_from(case.at.copy).unwrap();
        assert_eq!(entry(&mut model, case.id, names::ABILITY, list), wanted, "list {}->{}", case.from, case.to);
        assert_eq!(entry(&mut model, case.id, names::DICTIONARY, copy), wanted, "copy {}->{}", case.from, case.to);
        let content = wanted[1];
        let addition = u32::try_from(case.at.addition.unwrap()).unwrap();
        let sort = u32::try_from(case.at.sort.unwrap()).unwrap();
        assert_eq!(order_at(&mut model, case.id, names::ADDITION_ORDER, addition), content);
        assert_eq!(order_at(&mut model, case.id, names::SORT_ORDER, sort), content);
    }
}

/// The content ids of an order list that holds the spells `kept` of character 1.
fn order_without(kept: &[(u32, u32)], internal: bool) -> Vec<u32> {
    let mut ids = Vec::new();
    if internal {
        ids.push(content_of(711));
    }
    ids.extend(kept.iter().map(|(chain, level)| Spell::consistent(*chain, *level).content_id()));
    ids
}

/// The two order lists of character 1 and where the old content id sits in each.
struct State {
    label: &'static str,
    addition: Vec<u32>,
    sort: Vec<u32>,
    addition_at: Option<usize>,
    sort_at: Option<usize>,
}

#[test]
fn the_order_lists_in_three_states() {
    // Chain 0 at level 5 is the spell under edit; the lists drop its content id or keep it.
    let others = [(1, 1), (2, 16)];
    let all = [(0, 5), (1, 1), (2, 16)];
    let state = |label, addition, sort, addition_at, sort_at| State {
        label,
        addition,
        sort,
        addition_at,
        sort_at,
    };
    let states = [
        state("both", order_without(&all, true), order_without(&all, false), Some(1), Some(0)),
        state("addition only", order_without(&all, true), order_without(&others, false), Some(1), None),
        state("sort only", order_without(&others, true), order_without(&all, false), None, Some(0)),
        state("neither", order_without(&others, true), order_without(&others, false), None, None),
    ];
    for State {
        label,
        addition,
        sort,
        addition_at,
        sort_at,
    } in states
    {
        let mut slot = SlotData::standard();
        slot.characters[0].addition_order = Some(addition.clone());
        slot.characters[0].sort_order = Some(sort.clone());
        let mut model = model(&slot.render());
        let before = model.document().to_json();
        let change = model.apply_game(&level_edit(1, 0, 7)).unwrap();
        let at = Where {
            character: 0,
            list: 1,
            copy: 0,
            addition: addition_at,
            sort: sort_at,
        };
        let expected = at.changes(&PARTS);
        assert_changes(label, &diff(&before, &model.document().to_json()), &expected);
        assert_eq!(change.forward.len(), expected.len(), "{label}: one Set per changed place");
        // No order list grows or loses an element.
        let lengths = |model: &mut Ff2Model| {
            (0..8)
                .filter(|position| model.get(&names::order_place(1, names::ADDITION_ORDER, *position)).is_ok())
                .count()
        };
        assert_eq!(lengths(&mut model), addition.len(), "{label}");
    }
}

#[test]
fn the_same_level_again_changes_no_byte() {
    let mut model = standard_model();
    let before = model.document().to_json();
    let change = model.apply_game(&level_edit(1, 0, 5)).unwrap();
    assert_eq!(model.document().to_json(), before);
    // Only the first step stays when every stored value already equals its target.
    assert_eq!(change.forward.len(), 1);
    assert_eq!(change.forward.edits()[0].target(), &names::place(1, names::ABILITY, 1, names::ABILITY_ID));
    replay(&mut model, &change.inverse).unwrap();
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn a_mismatched_pair_is_repaired() {
    // Character 2 holds chain 5 at row level 1 with the values 456 (list) and 400 (copy).
    let at = Where {
        character: 1,
        list: 3,
        copy: 2,
        addition: Some(3),
        sort: Some(1),
    };
    let mut model = standard_model();
    assert_eq!(entry(&mut model, 2, names::ABILITY, 3)[2], 456);
    assert_eq!(entry(&mut model, 2, names::DICTIONARY, 2)[2], 400);
    let before = model.document().to_json();
    // The same row: only the two values move.
    let change = model.apply_game(&level_edit(2, 5, 1)).unwrap();
    assert_eq!(change.forward.len(), 3);
    let only_values = Where {
        addition: None,
        sort: None,
        ..at
    };
    assert_changes("same row", &diff(&before, &model.document().to_json()), &only_values.changes(&["skillLevel"]));
    let wanted = row(5, 1);
    assert_eq!(entry(&mut model, 2, names::ABILITY, 3), wanted);
    assert_eq!(entry(&mut model, 2, names::DICTIONARY, 2), wanted);
    assert_eq!(model.get(&names::spell_field(2, 5)).unwrap(), Value::Int(1));
    replay(&mut model, &change.inverse).unwrap();
    assert_eq!(model.document().to_json(), before, "undo returns to the mismatch");
    assert_eq!(entry(&mut model, 2, names::ABILITY, 3)[2], 456);
    assert_eq!(entry(&mut model, 2, names::DICTIONARY, 2)[2], 400);
    // Another row: all eight places.
    let change = model.apply_game(&level_edit(2, 5, 4)).unwrap();
    assert_eq!(change.forward.len(), 8);
    assert_changes("other row", &diff(&before, &model.document().to_json()), &at.changes(&PARTS));
    assert_eq!(entry(&mut model, 2, names::DICTIONARY, 2), row(5, 4));
}

#[test]
fn the_ends_of_a_chain_write_their_totals() {
    assert_eq!(tables().skill_total(1), Some(0));
    assert_eq!(tables().skill_total(16), Some(860));
    for (chain, level, total) in [(0, 1, 0), (0, 16, 860), (1, 16, 860), (2, 1, 0)] {
        let mut model = standard_model();
        model.apply_game(&level_edit(1, chain, level)).unwrap();
        let list = chain + 1;
        let level_u32 = u32::try_from(level).unwrap();
        assert_eq!(entry(&mut model, 1, names::ABILITY, list), row(chain, level_u32));
        assert_eq!(entry(&mut model, 1, names::ABILITY, list)[2], total);
        assert_eq!(model.get(&names::spell_field(1, chain)).unwrap(), Value::Int(level));
    }
}

#[test]
fn a_level_outside_one_to_sixteen_is_out_of_range() {
    for level in [0, 17, -1, 100] {
        let mut model = standard_model();
        let before = model.document().to_json();
        let error = model.apply_game(&level_edit(1, 0, level)).unwrap_err();
        assert!(
            matches!(error, EditError::OutOfRange { min: 1, max: 16, value, .. } if value == level),
            "{level}: {error:?}"
        );
        assert_eq!(model.document().to_json(), before, "{level}");
    }
}

#[test]
fn an_unowned_chain_and_a_missing_character_write_nothing() {
    let mut model = standard_model();
    let before = model.document().to_json();
    let unowned = model.apply_game(&level_edit(1, 3, 4)).unwrap_err();
    assert!(matches!(unowned, EditError::UnknownField { .. }), "{unowned:?}");
    let absent = model.apply_game(&level_edit(99, 0, 4)).unwrap_err();
    assert!(matches!(absent, EditError::MissingRow { .. }), "{absent:?}");
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn ability_711_and_the_other_characters_stay() {
    let mut model = standard_model();
    let internal = entry(&mut model, 1, names::ABILITY, 0);
    assert_eq!(internal[0], 711);
    let others: Vec<(FieldId, Value)> = (2..=4)
        .flat_map(|id| (0..40).map(move |chain| names::spell_field(id, chain)))
        .filter_map(|field| Some((field.clone(), model.get(&field).ok()?)))
        .collect();
    assert_ne!(others.len(), 0);
    model.apply_game(&level_edit(1, 0, 9)).unwrap();
    assert_eq!(entry(&mut model, 1, names::ABILITY, 0), internal);
    assert_eq!(model.get(&names::spell_field(1, 1)).unwrap(), Value::Int(1));
    for (field, value) in others {
        assert_eq!(model.get(&field).unwrap(), value, "{field}");
    }
}

#[test]
fn a_spaced_slot_undoes_up_to_layout() {
    let mut slot = SlotData::standard();
    slot.spaced = true;
    let text = slot.render();
    let mut history = History::open(&text);
    let before = history.text();
    history.edit(&level_edit(1, 0, 9)).unwrap();
    assert_ne!(history.text(), before);
    assert!(history.undo());
    // Only the whitespace of a rewritten level may differ after an undo.
    assert!(diff(&before, &history.text()).iter().all(|change| change.kind == Kind::Layout));
    assert!(history.undo.is_empty() && history.redo.len() == 1);
}

/// Tables whose chain holds three rows and whose curve holds two totals.
fn short_curve_tables() -> &'static GameTables {
    let mut abilities = String::new();
    for id in 1..=3 {
        let row = "type_id = 1\nkind = \"white_magic\"\nlevel = 0\n\n";
        write!(abilities, "[[ability]]\nid = {id}\ncontent_id = {}\n{row}", 100 + id).unwrap();
    }
    let skills = "[[status]]\nid = 1\nhp = 10\nmp = 5\npower = 1\nvitality = 2\nagility = 3\nintelligence = 4\nspirit = 5\nmagic = 6\n\n\
                  [skill_curve]\ntotal_exp = [0, 7]\n\n[spell_chains]\nfirst = [1]\nlength = 3\n";
    let sources = Sources {
        content: "[[content]]\nid = 5\nkind = \"item\"\nname_en = \"Ether\"\nname_fr = \"Éther\"\n",
        abilities: &abilities,
        jobs: "[[job]]\nid = 1\nname_en = \"Warrior\"\nname_fr = \"Guerrier\"\n",
        commands: "[[command]]\nid = 1\nname_en = \"Attack\"\nname_fr = \"Attaque\"\n",
        monsters: None,
        bounds: "[bounds]\nMAX_GIL = 999999\n",
        growth: None,
        skills: Some(skills),
    };
    // The model keeps the tables for its whole life: leak them for `'static`.
    Box::leak(Box::new(GameTables::parse(Game::Ff2Pr, &sources).unwrap()))
}

#[test]
fn a_level_without_a_total_is_unknown_and_writes_nothing() {
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    // The first spell of character 1 and its copy become row 1 of the short chain.
    let list = character_path(
        0,
        &[
            Segment::Doc,
            Segment::Key("abilityList"),
            Segment::Doc,
            Segment::Key("target"),
            Segment::Index(1),
            Segment::Doc,
            Segment::Key("abilityId"),
        ],
    );
    let copy = character_path(
        0,
        &[
            Segment::Doc,
            Segment::Key("abilityDictionary"),
            Segment::Doc,
            Segment::Key("values"),
            Segment::Index(0),
            Segment::Doc,
            Segment::Key("target"),
            Segment::Index(0),
            Segment::Doc,
            Segment::Key("abilityId"),
        ],
    );
    document.set(&list, &Scalar::Int(1)).unwrap();
    document.set(&copy, &Scalar::Int(1)).unwrap();
    let mut model = Ff2Model::new(document, short_curve_tables());
    let before = model.document().to_json();
    // Level 3 has a row but no total on the curve.
    let error = model.apply_game(&level_edit(1, 0, 3)).unwrap_err();
    assert!(matches!(error, EditError::UnknownId { id: 3, .. }), "{error:?}");
    assert_eq!(model.document().to_json(), before);
    // Level 2 has both.
    let change = model.apply_game(&level_edit(1, 0, 2)).unwrap();
    // The stored values differ from the short tables' rows, and both order lists hold the old content id.
    assert_eq!(change.forward.len(), 8);
}

#[test]
fn the_forward_list_follows_the_plan_order() {
    let mut model = standard_model();
    let change = model.apply_game(&level_edit(1, 0, 6)).unwrap();
    let expected: Vec<FieldId> = [
        names::place(1, names::ABILITY, 1, names::ABILITY_ID),
        names::place(1, names::ABILITY, 1, names::CONTENT_ID),
        names::place(1, names::ABILITY, 1, names::PROFICIENCY),
        names::place(1, names::DICTIONARY, 0, names::ABILITY_ID),
        names::place(1, names::DICTIONARY, 0, names::CONTENT_ID),
        names::place(1, names::DICTIONARY, 0, names::PROFICIENCY),
        names::order_place(1, names::ADDITION_ORDER, 1),
        names::order_place(1, names::SORT_ORDER, 2),
    ]
    .to_vec();
    let found: Vec<FieldId> = change.forward.edits().iter().map(|edit| edit.target().clone()).collect();
    assert_eq!(found, expected);
    // The inverse lists the old values in reverse order.
    let reversed: Vec<FieldId> = change.inverse.edits().iter().map(|edit| edit.target().clone()).collect();
    assert_eq!(reversed, expected.into_iter().rev().collect::<Vec<_>>());
}
