//! The spell cases of the FF II matrix: a Level edit up and down through the
//! game interface, and three one-sided edits of one stored place through the
//! raw interface (`docs/formats/pixel-remaster.md#row-and-value-in-ff-ii`).
//!
//! The Level oracle reads the places from the tree: the first entry of the
//! chain in `abilityList` and in the key-3 dictionary list, and the first
//! position of the old content id in each order list. It computes the target
//! of every place from the tables and expects a change at each place whose
//! stored value differs from its target, so a mismatched pair or a missing
//! order entry needs no special case.

use neoffse_core::{Capability, Edit, FieldId, Key, TableKind, Value};
use neoffse_format_pr::ff2::names;

use super::edit_proof::{How, Proof, ViewChange};
use super::exit_cases::{Case, nudged};
use super::exit_facts::{CHARS, Facts};
use super::exit_ff2::{Ff2Case, shape};
use super::json_diff::value;

/// Chains hold 16 levels (`docs/formats/pixel-remaster.md#spells-in-ff-ii`).
const TOP: u32 = 16;

/// The key of the Magic command in `abilityDictionary`.
const MAGIC_COMMAND: i64 = 3;

/// A spell of the save and where its places sit.
struct Found {
    id: u32,
    chain: u32,
    level: u32,
    /// The path of the character.
    base: String,
    /// The index of the first entry of the chain in `abilityList`.
    list: usize,
    /// The index of its copy in the key-3 dictionary list, with that list's path.
    copy: Option<(usize, String)>,
}

/// The first entry of `chain` in the array at `array`.
fn first_of_chain(facts: &Facts<'_>, array: &str, chain: u32) -> Option<usize> {
    let tree = &facts.subject.tree;
    (0..tree.array_len(array)?).find(|index| {
        tree.int(&format!("{array}[{index}].abilityId"))
            .and_then(|ability| u32::try_from(ability).ok())
            .and_then(|ability| facts.tables.spell_step(ability))
            .is_some_and(|step| step.chain == chain)
    })
}

/// The first Level field of the Magic section that `accept` takes, with the
/// places of its spell.
fn find(facts: &Facts<'_>, accept: impl Fn(&Found) -> bool) -> Option<Found> {
    let tree = &facts.subject.tree;
    let entities = &facts.view.section(Capability::Magic)?.entities;
    for field in entities.iter().flat_map(|entity| &entity.fields) {
        let ([Key::Name(names::CHARACTER), Key::Id(id), Key::Name(names::SPELL), Key::Id(chain)], Value::Int(level)) = (field.id.keys(), &field.value) else {
            continue;
        };
        let Some(index) = tree.find_row(CHARS, "id", i64::from(*id)) else {
            continue;
        };
        let base = format!("{CHARS}[{index}]");
        let Some(list) = first_of_chain(facts, &format!("{base}.abilityList.target"), *chain) else {
            continue;
        };
        let copy = tree.find_int(&format!("{base}.abilityDictionary.keys"), MAGIC_COMMAND).and_then(|values| {
            let array = format!("{base}.abilityDictionary.values[{values}].target");
            Some((first_of_chain(facts, &array, *chain)?, array))
        });
        let found = Found {
            id: *id,
            chain: *chain,
            level: u32::try_from(*level).ok()?,
            base,
            list,
            copy,
        };
        if accept(&found) {
            return Some(found);
        }
    }
    None
}

/// The first position of `content` in the order list at `array`.
fn order_position(facts: &Facts<'_>, array: &str, content: i64) -> Option<usize> {
    let tree = &facts.subject.tree;
    (0..tree.array_len(array)?).find(|index| tree.int(&format!("{array}[{index}]")) == Some(content))
}

/// One stored place a Level edit writes.
struct Place {
    path: String,
    field: FieldId,
    target: i64,
    /// True for the dictionary copy.
    copy: bool,
}

/// The places that a Level edit of `found` to `level` writes, in plan order.
/// `None` when the tables lack the row, its content id or its total.
fn places(facts: &Facts<'_>, found: &Found, level: u32) -> Option<Vec<Place>> {
    let tables = facts.tables;
    let tree = &facts.subject.tree;
    let ability = tables.spell_ability(found.chain, level)?;
    let content = i64::from(tables.entry(TableKind::Ability, ability)?.ability?.content_id?);
    let total = i64::from(tables.skill_total(level)?);
    let parts = [
        (names::ABILITY_ID, "abilityId", i64::from(ability)),
        (names::CONTENT_ID, "contentId", content),
        (names::PROFICIENCY, "skillLevel", total),
    ];
    let list = format!("{}.abilityList.target", found.base);
    let old_content = tree.int(&format!("{list}[{}].contentId", found.list))?;
    let mut places = Vec::new();
    let groups = [(names::ABILITY, list, found.list, false)]
        .into_iter()
        .chain(found.copy.iter().map(|(index, array)| (names::DICTIONARY, array.clone(), *index, true)));
    for (group, array, index, copy) in groups {
        for (part, key, target) in parts {
            places.push(Place {
                path: format!("{array}[{index}].{key}"),
                field: names::place(found.id, group, u32::try_from(index).ok()?, part),
                target,
                copy,
            });
        }
    }
    for (key, order) in [
        ("additionOrderOwnedAbilityIds", names::ADDITION_ORDER),
        ("sortOrderOwnedAbilityIds", names::SORT_ORDER),
    ] {
        let array = format!("{}.{key}.target", found.base);
        if let Some(index) = order_position(facts, &array, old_content) {
            places.push(Place {
                path: format!("{array}[{index}]"),
                field: names::order_place(found.id, order, u32::try_from(index).ok()?),
                target: content,
                copy: false,
            });
        }
    }
    Some(places)
}

/// The Level edit of `found` to `level`. With `copy` false the expected
/// change set leaves out the dictionary places: the proof must then fail.
fn level_case(facts: &Facts<'_>, name: &'static str, found: &Found, level: u32, copy: bool) -> Option<Ff2Case> {
    let tree = &facts.subject.tree;
    let mut proof = Proof {
        changes: Vec::new(),
        view: vec![ViewChange::Set(names::spell_field(found.id, found.chain), Value::Int(i64::from(level)))],
        texts: Vec::new(),
    };
    for place in places(facts, found, level)? {
        if tree.int(&place.path) == Some(place.target) {
            continue;
        }
        if copy || !place.copy {
            proof.changes.push(value(&place.path));
        }
        proof.view.push(ViewChange::Set(place.field, Value::Int(place.target)));
        proof.texts.push((place.path, place.target.to_string()));
    }
    Some(Ff2Case {
        name,
        edits: vec![Edit::set(names::spell_field(found.id, found.chain), Value::Int(i64::from(level)))],
        runs: vec![(How::Game, proof)],
    })
}

/// A Level case over the first spell the save holds with a dictionary copy,
/// whose expected change set omits that copy. The proof of it must fail on
/// every save that holds a spell.
pub(super) fn level_without_copy(facts: &Facts<'_>) -> Option<Ff2Case> {
    let found = find(facts, |found| found.copy.is_some())?;
    let level = if found.level < TOP { found.level + 1 } else { found.level - 1 };
    level_case(facts, "level_without_copy", &found, level, false)
}

/// The Level cases up and down, then one-sided edits of `proficiency`,
/// `ability_id` and the first place of the addition order.
pub(super) fn cases(facts: &Facts<'_>) -> Vec<(&'static str, Option<Ff2Case>)> {
    let has_copy = |found: &Found| found.copy.is_some();
    // A copy past index 0 tells a pairing by chain from one that always takes
    // the first copy; a save with no such spell falls back to any copy.
    let up = find(facts, |found| {
        has_copy(found) && found.level < TOP && found.copy.as_ref().is_some_and(|(index, _)| *index > 0)
    })
    .or_else(|| find(facts, |found| has_copy(found) && found.level < TOP));
    let down = find(facts, |found| has_copy(found) && found.level > 1);
    let one_sided = find(facts, |_| true);
    vec![
        ("spell_up", up.and_then(|found| level_case(facts, "spell_up", &found, found.level + 1, true))),
        (
            "spell_down",
            down.and_then(|found| level_case(facts, "spell_down", &found, found.level - 1, true)),
        ),
        ("place_proficiency", one_sided.as_ref().and_then(|found| proficiency(facts, found))),
        ("place_ability_id", one_sided.as_ref().and_then(|found| ability_id(facts, found))),
        ("place_order", one_sided.as_ref().and_then(|found| order(facts, found))),
    ]
}

/// A one-sided edit runs through the raw interface only, under `Enforce` and
/// under `StorageOnly`: the game interface refuses a place
/// (`Ff2Model::field_locked`).
fn raw_only(case: Option<Case>) -> Option<Ff2Case> {
    case.map(|case| shape(case, false, true))
}

/// `skillLevel` of the `abilityList` entry, with the row left alone: the
/// state that `docs/formats/pixel-remaster.md#row-and-value-in-ff-ii` closes.
fn proficiency(facts: &Facts<'_>, found: &Found) -> Option<Ff2Case> {
    let path = format!("{}.abilityList.target[{}].skillLevel", found.base, found.list);
    let field = names::place(found.id, names::ABILITY, u32::try_from(found.list).ok()?, names::PROFICIENCY);
    raw_only(nudged(facts, "place_proficiency", field, 0, path))
}

/// The first position of the addition order, a content id.
fn order(facts: &Facts<'_>, found: &Found) -> Option<Ff2Case> {
    let path = format!("{}.additionOrderOwnedAbilityIds.target[0]", found.base);
    raw_only(nudged(facts, "place_order", names::order_place(found.id, names::ADDITION_ORDER, 0), 0, path))
}

/// `abilityId` of the `abilityList` entry moved to the next row of its chain
/// (the previous one at the top), with the content id and the value left
/// alone. The Level field reads the row, so it changes with it.
fn ability_id(facts: &Facts<'_>, found: &Found) -> Option<Ff2Case> {
    let level = if found.level < TOP { found.level + 1 } else { found.level - 1 };
    let row = i64::from(facts.tables.spell_ability(found.chain, level)?);
    let path = format!("{}.abilityList.target[{}].abilityId", found.base, found.list);
    let field = names::place(found.id, names::ABILITY, u32::try_from(found.list).ok()?, names::ABILITY_ID);
    let proof = Proof {
        changes: vec![value(&path)],
        view: vec![
            ViewChange::Set(field.clone(), Value::Int(row)),
            ViewChange::Set(names::spell_field(found.id, found.chain), Value::Int(i64::from(level))),
        ],
        texts: vec![(path, row.to_string())],
    };
    Some(Ff2Case {
        name: "place_ability_id",
        edits: vec![Edit::set(field, Value::Int(row))],
        runs: vec![(How::Raw, proof.clone()), (How::Expert, proof)],
    })
}
