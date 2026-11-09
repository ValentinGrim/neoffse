//! The exit criterion of the FF II format on the synthetic slot: each edit
//! kind, through `Box<dyn SaveDocument>`, changes the file in the edited
//! tokens and nowhere else, on the compact text and on a respaced copy of it.
//! The generated matrix states its expected change lists from the save; the
//! anchors state theirs by hand.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_field;
use common::edit_proof::{How, Proof, Subject, ViewChange, prove};
use common::exit_ff2::{ALWAYS, Ff2Case, build, level_without_copy, run};
use common::ff2_ability_rules::ability_violations;
use common::ff2_rules::{cap_violations, violations};
use common::ff2_save::{SlotData, content_of, standard_model, tables};
use common::ff2_spell::{Where, level_edit};
use common::json_diff::{added, removed, value};
use common::json_tree::Tree;
use neoffse_core::{CapPolicy, Edit, FieldId, RowId, TableKind, Value};
use neoffse_format_pr::container::Container;
use neoffse_format_pr::ff2::{Ff2Format, names};

const WORDS: &str = "userData.ownedKeyWaordList.target";

/// The cases the synthetic slot cannot build, in matrix order. Its first
/// character wears the same weapon in both hands, so no case lacks an
/// ingredient there; the list stays as a guard against a silent skip.
const FIXTURE_SKIPS: [&str; 0] = [];

fn subject() -> Subject {
    Subject::from_json(Ff2Format::new(), SlotData::standard().render(), Container::game())
}

fn keyword(key: u32) -> RowId {
    RowId::new(FieldId::new(names::KEYWORDS), key)
}

fn case_named<'a>(cases: &'a [Ff2Case], name: &str) -> &'a Ff2Case {
    cases.iter().find(|case| case.name == name).unwrap()
}

fn prove_matrix(subject: &Subject) {
    let built = build(subject);
    assert!(built.skipped == FIXTURE_SKIPS, "cases not built: {:?}", built.skipped);
    for name in ALWAYS {
        assert!(built.cases.iter().any(|case| case.name == name), "the case {name} is missing");
    }
    for case in &built.cases {
        run(subject, case);
    }
}

#[test]
fn every_generated_case_proves_on_the_fixture() {
    prove_matrix(&subject());
}

#[test]
fn every_generated_case_proves_on_the_respaced_fixture() {
    let spaced = subject().respaced(4);
    assert!(spaced.json != spaced.tree.spaced(0), "the text is not spaced");
    prove_matrix(&spaced);
}

#[test]
fn the_matrix_covers_every_edit_kind() {
    let built = build(&subject());
    let names: Vec<_> = built.cases.iter().map(|case| case.name).collect();
    for wanted in [
        "equip_new",
        "equip_new_s2",
        "equip_new_s3",
        "equip_new_s4",
        "equip_new_s5",
        "unequip",
        "unequip_s2",
        "spell_up",
        "spell_down",
        "place_proficiency",
        "place_ability_id",
        "place_order",
        "keyword_add",
        "keyword_remove",
        "item_count",
        "remove_item",
        "remove_key_item",
        "compose",
    ] {
        assert!(names.contains(&wanted), "the case {wanted} is missing");
    }
    for (name, hows) in [
        ("max_hp_gain", vec![How::Game, How::Raw, How::Expert]),
        ("place_order", vec![How::Raw, How::Expert]),
        ("keyword_remove", vec![How::Raw, How::Expert]),
        ("spell_up", vec![How::Game]),
    ] {
        let found: Vec<_> = case_named(&built.cases, name).runs.iter().map(|(how, _)| *how).collect();
        assert_eq!(found, hows, "{name}");
    }
}

#[test]
fn a_level_edit_changes_the_eight_places_and_nothing_else() {
    // Character 1 holds ability 711, then chain 0 at level 5 (`ff2_spell_edit`).
    let at = Where {
        character: 0,
        list: 1,
        copy: 0,
        addition: Some(1),
        sort: Some(2),
    };
    let to = 6;
    let ability = tables().spell_ability(0, to).unwrap();
    let content = content_of(ability);
    let total = i64::from(tables().skill_total(to).unwrap());
    let base = "userData.ownedCharacterList.target[0]";
    let places = [
        (
            format!("{base}.abilityList.target[1].abilityId"),
            names::place(1, names::ABILITY, 1, names::ABILITY_ID),
            i64::from(ability),
        ),
        (
            format!("{base}.abilityList.target[1].contentId"),
            names::place(1, names::ABILITY, 1, names::CONTENT_ID),
            i64::from(content),
        ),
        (
            format!("{base}.abilityList.target[1].skillLevel"),
            names::place(1, names::ABILITY, 1, names::PROFICIENCY),
            total,
        ),
        (
            format!("{base}.abilityDictionary.values[0].target[0].abilityId"),
            names::place(1, names::DICTIONARY, 0, names::ABILITY_ID),
            i64::from(ability),
        ),
        (
            format!("{base}.abilityDictionary.values[0].target[0].contentId"),
            names::place(1, names::DICTIONARY, 0, names::CONTENT_ID),
            i64::from(content),
        ),
        (
            format!("{base}.abilityDictionary.values[0].target[0].skillLevel"),
            names::place(1, names::DICTIONARY, 0, names::PROFICIENCY),
            total,
        ),
        (
            format!("{base}.additionOrderOwnedAbilityIds.target[1]"),
            names::order_place(1, names::ADDITION_ORDER, 1),
            i64::from(content),
        ),
        (
            format!("{base}.sortOrderOwnedAbilityIds.target[2]"),
            names::order_place(1, names::SORT_ORDER, 2),
            i64::from(content),
        ),
    ];
    let mut view = vec![ViewChange::Set(names::spell_field(1, 0), Value::Int(i64::from(to)))];
    view.extend(places.iter().map(|(_, field, target)| ViewChange::Set(field.clone(), Value::Int(*target))));
    let proof = Proof {
        changes: at.changes(&["abilityId", "contentId", "skillLevel"]),
        view,
        texts: places.iter().map(|(path, _, target)| (path.clone(), target.to_string())).collect(),
    };
    prove(&subject(), "level anchor", &[level_edit(1, 0, i64::from(to))], How::Game, &proof);
}

#[test]
fn a_keyword_is_appended_and_a_removal_returns_to_its_index() {
    let subject = subject();
    let listed: Vec<u32> = common::ff2_save::keywords();
    let key = common::ff2_save::ids(TableKind::Keyword).into_iter().find(|key| !listed.contains(key)).unwrap();
    let path = format!("{WORDS}[{}]", listed.len());
    let add = Proof {
        changes: vec![added(&path)],
        view: vec![ViewChange::Listed {
            row: keyword(key),
            position: listed.len(),
        }],
        texts: vec![(path, key.to_string())],
    };
    let insert = Edit::insert(keyword(key), None, Vec::new());
    prove(&subject, "keyword add", std::slice::from_ref(&insert), How::Game, &add);
    // The first listed keyword leaves from the front; the inverse puts it back there.
    let first = listed[0];
    let take = Proof {
        changes: vec![removed(&format!("{WORDS}[0]"))],
        view: vec![ViewChange::Remove(keyword(first))],
        texts: Vec::new(),
    };
    prove(&subject, "keyword remove", &[Edit::remove(keyword(first))], How::Expert, &take);
    let change = subject
        .open()
        .apply(&Edit::remove(keyword(first)), neoffse_core::CapPolicy::StorageOnly)
        .unwrap();
    let Edit::Insert { position, .. } = change else {
        panic!("the inverse of a remove is one insert")
    };
    assert_eq!(position, Some(0));
}

#[test]
fn a_game_removal_of_a_keyword_is_refused_and_writes_nothing() {
    let subject = subject();
    let mut document = subject.open();
    let first = common::ff2_save::keywords()[0];
    let result = document.apply_game(&Edit::remove(keyword(first)));
    assert!(matches!(result, Err(neoffse_core::EditError::ReadOnly { .. })));
    let files = document.encode().unwrap();
    let after = neoffse_format_pr::container::decode(files.get(&subject.name).unwrap()).unwrap();
    assert!(after.json == subject.json, "the refused removal changed the text");
}

#[test]
#[should_panic(expected = "unexpected change")]
fn a_level_proof_without_the_dictionary_copy_fails() {
    let subject = subject();
    let case = level_without_copy(&subject).unwrap();
    run(&subject, &case);
}

#[test]
#[should_panic(expected = "missing change")]
fn a_level_proof_with_a_stray_place_fails() {
    let subject = subject();
    let built = build(&subject);
    let case = case_named(&built.cases, "spell_up");
    let mut proof = case.runs[0].1.clone();
    proof.changes.push(value("userData.playTime"));
    prove(&subject, "stray place", &case.edits, How::Game, &proof);
}

#[test]
fn the_cap_check_passes_the_fixture_and_flags_a_gain_past_its_room() {
    let mut clean = standard_model();
    assert_eq!(cap_violations(&mut clean), Vec::<String>::new());
    assert_eq!(violations(&mut clean, true), Vec::<String>::new());
    // Character 1 has status 1: any gain above `cap - base` is a breach.
    let room = {
        let base = tables().base_stats(1).unwrap();
        tables().bound("MAX_STRNGTH").unwrap() - i64::from(base.power)
    };
    for (stat, gain) in [(names::POWER, room + 1), (names::MAX_HP_GAIN, 10_000), (names::MAX_MP_GAIN, 1_000)] {
        let mut broken = standard_model();
        let edit = Edit::set(character_field(1, stat), Value::Int(gain));
        broken.apply(&edit, CapPolicy::StorageOnly).unwrap();
        assert_eq!(cap_violations(&mut broken).len(), 1, "{stat}");
    }
    let mut at_room = standard_model();
    at_room
        .apply(&Edit::set(character_field(1, names::POWER), Value::Int(room)), CapPolicy::StorageOnly)
        .unwrap();
    assert_eq!(cap_violations(&mut at_room), Vec::<String>::new());
}

#[test]
fn the_cap_check_flags_a_skill_and_a_proficiency_past_their_cap() {
    for field in [
        character_field(1, names::SKILL).with_id(0),
        names::place(1, names::ABILITY, 1, names::PROFICIENCY),
    ] {
        let mut broken = standard_model();
        broken.apply(&Edit::set(field.clone(), Value::Int(861)), CapPolicy::StorageOnly).unwrap();
        assert_eq!(cap_violations(&mut broken), [format!("cap: {field} is above its cap")]);
    }
}

#[test]
fn the_ability_check_passes_the_fixture_and_flags_a_broken_order_list() {
    let text = SlotData::standard().render();
    assert_eq!(ability_violations(&Tree::parse(&text)), Vec::<String>::new());
    let mut data = SlotData::standard();
    data.characters[0].addition_order = Some(vec![1, 2]);
    data.characters[1].sort_order = Some(vec![999_999]);
    let found = ability_violations(&Tree::parse(&data.render()));
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].contains("target[0]") && found[0].contains("additionOrder"));
    assert!(found[1].contains("target[1]") && found[1].contains("sortOrder"));
}

fn inventory_count(key: u32) -> FieldId {
    FieldId::new(names::INVENTORY).with_id(key).with_name(names::COUNT)
}

#[test]
fn the_floor_exempts_placeholder_rows_and_keeps_every_other_row() {
    // Two hands hold 47 (characters 1 and 3), and the row of 47 counts 1.
    let mut placeholder = standard_model();
    let hand = character_field(1, names::EQUIPMENT).with_id(1);
    placeholder.apply(&Edit::set(hand, Value::Int(47)), CapPolicy::StorageOnly).unwrap();
    placeholder
        .apply(&Edit::set(inventory_count(47), Value::Int(1)), CapPolicy::StorageOnly)
        .unwrap();
    assert_eq!(violations(&mut placeholder, false), Vec::<String>::new());
    // Character 1 wears 60 in both hands and the row counts 3: a count of 1 is a breach.
    let mut worn = standard_model();
    worn.apply(&Edit::set(inventory_count(60), Value::Int(1)), CapPolicy::StorageOnly).unwrap();
    let found = violations(&mut worn, false);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .all(|line| line.starts_with("inventory: character.1.equipment.") && line.ends_with("is worn more often than owned"))
    );
}
