//! The edit matrix of the FF II exit proof. Every choice is a function of the
//! save (its view, its tree and the FF II tables), so the same code runs on
//! the synthetic slot and on every private slot, and no value of a private
//! save is printed. The expected change sets follow
//! `docs/formats/pixel-remaster.md#documents-inside-strings`,
//! `#character-growth-in-ff-ii`, `#skill-levels-in-ff-ii`,
//! `#row-and-value-in-ff-ii` and `#keywords-in-ff-ii`.
//!
//! The item, equipment and name cases come from `exit_cases` and
//! `exit_equip`, which both games share. The spell cases live in
//! `exit_ff2_spell`.

use neoffse_core::{Edit, FieldId, RowId, TableKind};
use neoffse_format_pr::ff2::names;

use super::edit_proof::{How, Proof, Subject, ViewChange, prove};
use super::exit_cases::{Built, Case, compose, gil_case, item_cases, nudged, rename};
use super::exit_equip::{FF2_SLOTS, equip_cases};
use super::exit_facts::Facts;
use super::json_diff::{added, removed};

/// The cases every FF II slot must allow. The spell, place, equipment and
/// keyword cases depend on the save and may skip.
pub(crate) const ALWAYS: [&str; 20] = [
    "gil",
    "name",
    "hp",
    "mp",
    "max_hp_gain",
    "max_mp_gain",
    "stat",
    "skill_0",
    "skill_1",
    "skill_2",
    "skill_3",
    "skill_4",
    "skill_5",
    "skill_6",
    "skill_7",
    "skill_8",
    "skill_9",
    "insert_item_middle",
    "insert_item_append",
    "insert_key_item",
];

const SKILL_CASES: [&str; 10] = [
    "skill_0", "skill_1", "skill_2", "skill_3", "skill_4", "skill_5", "skill_6", "skill_7", "skill_8", "skill_9",
];

/// The cases that also run under `StorageOnly`, the policy of expert mode:
/// the gains, whose cap depends on the character in the game interface and on
/// nothing in the raw one.
const EXPERT: [&str; 3] = ["max_hp_gain", "max_mp_gain", "stat"];

/// `userData.ownedKeyWaordList.target`, the game's spelling.
const WORDS: &str = "userData.ownedKeyWaordList.target";

/// One edit case with the interfaces it runs through.
#[derive(Debug, Clone)]
pub(crate) struct Ff2Case {
    pub name: &'static str,
    pub edits: Vec<Edit>,
    /// Each interface the case runs through, with the proof it owes there.
    pub runs: Vec<(How, Proof)>,
}

/// The case of `case` with its runs: `apply_game` when `game` (under its
/// linked proof if it has one), `apply` under `Enforce` when the case allows
/// it, and `apply` under `StorageOnly` when `expert`.
pub(super) fn shape(case: Case, game: bool, expert: bool) -> Ff2Case {
    let mut runs = Vec::new();
    if game {
        runs.push((How::Game, case.linked.clone().unwrap_or_else(|| case.proof.clone())));
    }
    if case.raw {
        runs.push((How::Raw, case.proof.clone()));
    }
    if expert {
        runs.push((How::Expert, case.proof));
    }
    Ff2Case {
        name: case.name,
        edits: case.edits,
        runs,
    }
}

fn skill(facts: &Facts<'_>, key: u32) -> Option<Case> {
    let list = format!("{}.skillLevelTargets", facts.char_path);
    let index = facts.subject.tree.find_int(&format!("{list}.keys"), i64::from(key))?;
    let name = SKILL_CASES.get(usize::try_from(key).ok()?)?;
    nudged(facts, name, facts.field(names::SKILL).with_id(key), 0, format!("{list}.values[{index}]"))
}

fn keyword_row(key: u32) -> RowId {
    RowId::new(FieldId::new(names::KEYWORDS), key)
}

/// The keywords in document order.
fn listed_keywords(facts: &Facts<'_>) -> Vec<u32> {
    let tree = &facts.subject.tree;
    (0..tree.array_len(WORDS).unwrap_or(0))
        .filter_map(|index| tree.int(&format!("{WORDS}[{index}]")))
        .filter_map(|key| u32::try_from(key).ok())
        .collect()
}

/// The game interface appends a keyword, as `AddKeyWord` does, and the raw
/// interface does the same under `Enforce`
/// (`docs/formats/pixel-remaster.md#keywords-in-ff-ii`).
fn keyword_add(facts: &Facts<'_>) -> Option<Ff2Case> {
    let listed = listed_keywords(facts);
    let key = facts.first_unowned(TableKind::Keyword, &listed)?;
    let path = format!("{WORDS}[{}]", facts.subject.tree.array_len(WORDS)?);
    let proof = Proof {
        changes: vec![added(&path)],
        view: vec![ViewChange::Listed {
            row: keyword_row(key),
            position: listed.len(),
        }],
        texts: vec![(path, key.to_string())],
    };
    Some(Ff2Case {
        name: "keyword_add",
        edits: vec![Edit::insert(keyword_row(key), None, Vec::new())],
        runs: vec![(How::Game, proof.clone()), (How::Raw, proof)],
    })
}

/// An expert removal of the keyword in the middle of the list: no game path
/// removes one, so only the raw interface runs it, and its inverse puts the
/// element back at the same index.
fn keyword_remove(facts: &Facts<'_>) -> Option<Ff2Case> {
    let listed = listed_keywords(facts);
    let index = listed.len() / 2;
    let key = *listed.get(index)?;
    let proof = Proof {
        changes: vec![removed(&format!("{WORDS}[{index}]"))],
        view: vec![ViewChange::Remove(keyword_row(key))],
        texts: Vec::new(),
    };
    Some(Ff2Case {
        name: "keyword_remove",
        edits: vec![Edit::remove(keyword_row(key))],
        runs: vec![(How::Raw, proof.clone()), (How::Expert, proof)],
    })
}

/// Every case the save allows, and the names of the ones it does not.
pub(crate) fn build(subject: &Subject) -> Built<Ff2Case> {
    let facts = Facts::read(subject);
    let char_path = &facts.char_path;
    let parameter = |part: &str| format!("{char_path}.parameter.{part}");
    let scalar = |name: &'static str, field: &'static str, key: &str| nudged(&facts, name, facts.field(field), 0, parameter(key));
    let mut singles = vec![
        gil_case(&facts),
        ("name", rename(&facts)),
        ("hp", scalar("hp", names::HP, "currentHP")),
        ("mp", scalar("mp", names::MP, "currentMP")),
        ("max_hp_gain", scalar("max_hp_gain", names::MAX_HP_GAIN, "addtionalMaxHp")),
        ("max_mp_gain", scalar("max_mp_gain", names::MAX_MP_GAIN, "addtionalMaxMp")),
        ("stat", scalar("stat", names::POWER, "addtionalPower")),
    ];
    singles.extend(item_cases(&facts));
    singles.extend((0..names::SKILLS).zip(SKILL_CASES).map(|(key, name)| (name, skill(&facts, key))));
    singles.extend(equip_cases(&facts, names::EQUIPMENT_SLOTS, &FF2_SLOTS));
    let find = |wanted: &str| singles.iter().find(|(name, _)| *name == wanted).and_then(|(_, case)| case.as_ref());
    let composed = compose(&[find("gil"), find("name"), find("insert_item_middle"), find("remove_key_item")]);
    singles.push(("compose", composed));
    let mut built = Built::new();
    built.extend(
        singles
            .into_iter()
            .map(|(name, case)| (name, case.map(|case| shape(case, true, EXPERT.contains(&name))))),
    );
    let others = [("keyword_add", keyword_add(&facts)), ("keyword_remove", keyword_remove(&facts))];
    built.extend(others.into_iter().chain(super::exit_ff2_spell::cases(&facts)));
    built
}

/// Proves `case` through each of its interfaces.
pub(crate) fn run(subject: &Subject, case: &Ff2Case) {
    for (how, proof) in &case.runs {
        prove(subject, case.name, &case.edits, *how, proof);
    }
}

/// The Level case whose expected change set omits the dictionary copy. Its
/// proof must fail on a save that holds a spell with a copy
/// (`docs/formats/pixel-remaster.md#row-and-value-in-ff-ii`).
pub(crate) fn level_without_copy(subject: &Subject) -> Option<Ff2Case> {
    super::exit_ff2_spell::level_without_copy(&Facts::read(subject))
}
