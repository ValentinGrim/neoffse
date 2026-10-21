//! The edit matrix of the M3 exit proof. Every choice is a function of the
//! save (its view, its tree and the FF I tables), so the same code runs on the
//! synthetic slot and on every private slot, and no value of a private save is
//! printed. The expected change sets follow `docs/formats/pixel-remaster.md`
//! (`#documents-inside-strings`, `#characters`, `#inventory`).
//!
//! The matrix leaves out a `Set` of `key_items.<K>.count`: `apply_game`
//! refuses it, and the model tests cover the refusal.

use neoffse_core::{Edit, FieldId, RowId, TableKind, Value};
use neoffse_format_pr::ff1::names;

use super::edit_proof::{How, Proof, Subject, ViewChange, prove};
use super::exit_facts::{Facts, INV, KEYS};
use super::json_diff::{added, removed, value};

/// Four characters: a non-ASCII letter, a quote and a backslash.
pub(crate) const NEW_NAME: &str = "Nö\"\\";

/// The cases every FF I slot must allow. Every other case depends on the
/// save (a free item, a worn item, a placeholder, a spell level) and may skip.
pub(crate) const ALWAYS: [&str; 8] = [
    "gil",
    "level",
    "stat",
    "job",
    "name",
    "insert_item_middle",
    "insert_item_append",
    "insert_key_item",
];

/// The cases the synthetic fixture cannot build, in matrix order: its first
/// character already wears the shield placeholder (nothing to unequip in slot
/// 2) and the inventory holds no gloves besides the worn ones and the
/// placeholder (nothing to equip from the inventory in slot 5).
pub(crate) const FIXTURE_SKIPS: [&str; 2] = ["unequip_s2", "equip_held_s5"];

#[derive(Debug, Clone)]
pub(crate) struct Case {
    pub name: &'static str,
    pub edits: Vec<Edit>,
    pub proof: Proof,
    /// Also run through `apply` under `Enforce`.
    pub raw: bool,
}

pub(crate) struct Built {
    pub cases: Vec<Case>,
    /// Cases whose ingredient the save lacks.
    pub skipped: Vec<&'static str>,
}

pub(super) fn case(name: &'static str, edits: Vec<Edit>, proof: Proof) -> Case {
    Case { name, edits, proof, raw: true }
}

/// `old - 1` when `old > min`, else `old + 1`.
fn nudge(old: i64, min: i64) -> i64 {
    if old > min { old - 1 } else { old + 1 }
}

/// A `Set` of an integer field to a nudged value, expected as one changed
/// token. `None` when the field is absent or the nudged value exceeds its cap
/// (an old value above the cap).
pub(super) fn nudged(facts: &Facts<'_>, name: &'static str, field: FieldId, min: i64, path: String) -> Option<Case> {
    let new = nudge(facts.int(&field)?, min);
    if facts.view.find(&field)?.1.meta.cap.is_some_and(|cap| new > cap) {
        return None;
    }
    let proof = Proof {
        changes: vec![value(&path)],
        view: vec![ViewChange::Set(field.clone(), Value::Int(new))],
        texts: vec![(path, new.to_string())],
    };
    Some(case(name, vec![Edit::set(field, Value::Int(new))], proof))
}

fn job(facts: &Facts<'_>) -> Option<Case> {
    let field = facts.field(names::JOB);
    let old = facts.int(&field)?;
    let new = facts.tables.table(TableKind::Job)?.iter().find(|entry| i64::from(entry.id) != old)?.id;
    let path = format!("{}.jobId", facts.char_path);
    let proof = Proof {
        changes: vec![value(&path)],
        view: vec![ViewChange::Set(field.clone(), Value::Int(i64::from(new)))],
        texts: vec![(path, new.to_string())],
    };
    Some(case("job", vec![Edit::set(field, Value::Int(i64::from(new)))], proof))
}

fn rename(facts: &Facts<'_>) -> Option<Case> {
    let field = facts.field(names::NAME);
    let Value::Text(old) = &facts.view.find(&field)?.1.value else {
        return None;
    };
    let new = if old == NEW_NAME { format!("{NEW_NAME}x") } else { NEW_NAME.to_owned() };
    let path = format!("{}.name", facts.char_path);
    let proof = Proof {
        changes: vec![value(&path)],
        view: vec![ViewChange::Set(field.clone(), Value::Text(new.clone()))],
        texts: vec![(path, serde_json::Value::String(new.clone()).to_string())],
    };
    Some(case("name", vec![Edit::set(field, Value::Text(new))], proof))
}

fn charges(facts: &Facts<'_>) -> Option<Case> {
    let (level, field) = (1..=names::SPELL_LEVELS)
        .map(|level| (level, facts.field(names::CHARGES).with_id(level)))
        .find(|(_, field)| facts.view.find(field).is_some())?;
    let list = format!("{}.parameter.currentMpCountList", facts.char_path);
    let index = facts.subject.tree.find_int(&format!("{list}.keys"), i64::from(level))?;
    nudged(facts, "charges", field, 0, format!("{list}.values[{index}]"))
}

fn item_count(facts: &Facts<'_>) -> Option<Case> {
    let key = facts.free_item?;
    let index = facts.subject.tree.find_row(INV, "contentId", i64::from(key))?;
    nudged(
        facts,
        "item_count",
        Facts::inventory(key).field(names::COUNT),
        1,
        format!("{INV}[{index}].count"),
    )
}

/// An `Insert` of `id` with `count` at `position` (`None`: the end) of a list
/// that holds `len` rows.
fn insert(name: &'static str, row: RowId, array: &str, len: usize, position: Option<usize>, count: i64) -> Case {
    let index = position.unwrap_or(len);
    let path = format!("{array}[{index}]");
    let text = format!("{{\"contentId\":{},\"count\":{count}}}", row.key());
    let proof = Proof {
        changes: vec![added(&path)],
        view: vec![ViewChange::Insert {
            row: row.clone(),
            position: index,
            count,
        }],
        texts: vec![(path, text)],
    };
    case(name, vec![Edit::insert(row, position, vec![(names::COUNT, Value::Int(count))])], proof)
}

fn insert_item(facts: &Facts<'_>, name: &'static str, position: Option<usize>) -> Option<Case> {
    let id = facts.first_unowned(TableKind::Item, &facts.inv_rows)?;
    Some(insert(name, Facts::inventory(id), INV, facts.inv_len, position, 3))
}

fn insert_key_item(facts: &Facts<'_>) -> Option<Case> {
    let id = facts.first_unowned(TableKind::KeyItem, &facts.key_rows)?;
    Some(insert("insert_key_item", Facts::key_item(id), KEYS, facts.keys_len, None, 1))
}

fn remove(facts: &Facts<'_>, name: &'static str, row: RowId, array: &str) -> Option<Case> {
    let index = facts.subject.tree.find_row(array, "contentId", i64::from(row.key()))?;
    let proof = Proof {
        changes: vec![removed(&format!("{array}[{index}]"))],
        view: vec![ViewChange::Remove(row.clone())],
        texts: Vec::new(),
    };
    Some(case(name, vec![Edit::remove(row)], proof))
}

/// The four cases applied in one run: edits in two arrays at once.
fn compose(parts: &[Option<&Case>]) -> Option<Case> {
    let mut joined = Case {
        name: "compose",
        edits: Vec::new(),
        proof: Proof {
            changes: Vec::new(),
            view: Vec::new(),
            texts: Vec::new(),
        },
        raw: true,
    };
    for part in parts {
        let part = (*part)?;
        joined.edits.extend(part.edits.iter().cloned());
        joined.proof.changes.extend(part.proof.changes.iter().cloned());
        joined.proof.view.extend(part.proof.view.iter().cloned());
        joined.proof.texts.extend(part.proof.texts.iter().cloned());
    }
    Some(joined)
}

/// Every case the save allows, and the names of the ones it does not.
pub(crate) fn build(subject: &Subject) -> Built {
    let facts = Facts::read(subject);
    let char_path = &facts.char_path;
    let parameter = |part: &str| format!("{char_path}.parameter.{part}");
    let gil = FieldId::new(names::GIL);
    let mut singles = vec![
        ("gil", nudged(&facts, "gil", gil, 0, "userData.owendGil".to_owned())),
        ("level", nudged(&facts, "level", facts.field(names::LEVEL), 1, parameter("addtionalLevel"))),
        ("stat", nudged(&facts, "stat", facts.field(names::POWER), 0, parameter("addtionalPower"))),
        ("job", job(&facts)),
        ("name", rename(&facts)),
        ("charges", charges(&facts)),
        ("item_count", item_count(&facts)),
        ("insert_item_middle", insert_item(&facts, "insert_item_middle", Some(facts.inv_len / 2))),
        ("insert_item_append", insert_item(&facts, "insert_item_append", None)),
        ("insert_key_item", insert_key_item(&facts)),
        (
            "remove_item",
            facts.free_item.and_then(|key| remove(&facts, "remove_item", Facts::inventory(key), INV)),
        ),
        (
            "remove_key_item",
            facts.free_key.and_then(|key| remove(&facts, "remove_key_item", Facts::key_item(key), KEYS)),
        ),
    ];
    singles.extend(super::exit_fields::field_cases(&facts));
    singles.extend(super::exit_equip::equip_cases(&facts));
    let find = |wanted: &str| singles.iter().find(|(name, _)| *name == wanted).and_then(|(_, case)| case.as_ref());
    let composed = compose(&[find("gil"), find("level"), find("insert_item_middle"), find("remove_key_item")]);
    singles.push(("compose", composed));
    let mut built = Built {
        cases: Vec::new(),
        skipped: Vec::new(),
    };
    for (name, case) in singles {
        match case {
            Some(case) => built.cases.push(case),
            None => built.skipped.push(name),
        }
    }
    built
}

/// `prove` with `How::Game`, then with `How::Raw` when the case allows it.
pub(crate) fn run(subject: &Subject, case: &Case) {
    prove(subject, case.name, &case.edits, How::Game, &case.proof);
    if case.raw {
        prove(subject, case.name, &case.edits, How::Raw, &case.proof);
    }
}
