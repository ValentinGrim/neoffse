//! The `level_up` case of the edit matrix: a level edit that raises the level
//! by one under the game interface, with every growth field it moves. The
//! expected values come from `growth_oracle` with all rolls failing, and from
//! the character's own values, so the same code runs on the fixture and on
//! every private slot (`docs/formats/pixel-remaster.md#level-up-in-ff-i`).

use neoffse_core::{Edit, FieldId, Value};
use neoffse_format_pr::ff1::names;

use super::edit_proof::{Proof, ViewChange};
use super::exit_cases::Case;
use super::exit_facts::Facts;
use super::growth_oracle::{Stored, grow};
use super::json_diff::{Change, value};

/// `(field, key of `parameter`)` of the scalar growth fields.
const KEYS: [(&str, &str); 10] = [
    (names::MAX_HP_GAIN, "addtionalMaxHp"),
    (names::POWER, "addtionalPower"),
    (names::VITALITY, "addtionalVitality"),
    (names::AGILITY, "addtionalAgility"),
    (names::INTELLIGENCE, "addtionalIntelligence"),
    (names::SPIRIT, "addtionalSpirit"),
    (names::LUCK, "addtionalLuck"),
    (names::MAGIC, "addtionalMagic"),
    (names::ACCURACY_GAIN, "addtionalAccuracyRate"),
    (names::MAGIC_EVASION_GAIN, "addtionalAbilityEvasionRate"),
];

/// The character's growth fields as the view holds them.
fn start(facts: &Facts<'_>) -> Option<Stored> {
    let field = |name| facts.int(&facts.field(name));
    let mut charges = [0; 8];
    for (level, held) in (1..=names::SPELL_LEVELS).zip(&mut charges) {
        *held = facts.int(&facts.field(names::CHARGES_MAX_GAIN).with_id(level))?;
    }
    Some(Stored {
        max_hp_gain: field(names::MAX_HP_GAIN)?,
        power: field(names::POWER)?,
        vitality: field(names::VITALITY)?,
        agility: field(names::AGILITY)?,
        intelligence: field(names::INTELLIGENCE)?,
        spirit: field(names::SPIRIT)?,
        luck: field(names::LUCK)?,
        magic: field(names::MAGIC)?,
        accuracy: field(names::ACCURACY_GAIN)?,
        evasion: field(names::MAGIC_EVASION_GAIN)?,
        charges,
    })
}

/// The document path of a growth field of the first character.
fn path_of(facts: &Facts<'_>, field: &FieldId, scalar: Option<&str>) -> Option<String> {
    let parameter = format!("{}.parameter", facts.char_path);
    if let Some(key) = scalar {
        return Some(format!("{parameter}.{key}"));
    }
    let list = format!("{parameter}.addtionalMaxMpCountList");
    let level = match field.keys().last()? {
        neoffse_core::Key::Id(level) => i64::from(*level),
        neoffse_core::Key::Name(_) => return None,
    };
    let index = facts.subject.tree.find_int(&format!("{list}.keys"), level)?;
    Some(format!("{list}.values[{index}]"))
}

/// `Set level <old + 1>`, run through `How::Rolled` with all rolls failing.
/// `None` for a level of 99, a job without a growth row or a missing key.
pub(super) fn level_up(facts: &Facts<'_>) -> Option<Case> {
    let level_field = facts.field(names::LEVEL);
    let old = u32::try_from(facts.int(&level_field)?).ok().filter(|old| *old < 99)?;
    let new = old + 1;
    let job = u32::try_from(facts.int(&facts.field(names::JOB))?).ok()?;
    facts.tables.growth_step(job, new)?;
    let status = facts.subject.tree.int(&format!("{}.characterStatusId", facts.char_path))?;
    let base_vitality = facts.tables.base_vitality(u32::try_from(status).ok()?)?;
    let before = start(facts)?;
    let after = grow(facts.tables, &before, base_vitality, job, old, new, false);

    let mut changes: Vec<Change> = Vec::new();
    let mut view = Vec::new();
    let mut texts = Vec::new();
    let mut moved = |path: String, field: FieldId, number: i64| {
        changes.push(value(&path));
        view.push(ViewChange::Set(field, Value::Int(number)));
        texts.push((path, number.to_string()));
    };
    moved(format!("{}.parameter.addtionalLevel", facts.char_path), level_field.clone(), i64::from(new));
    let first = i64::from(facts.tables.exp_to_reach(new)?);
    let exp = facts.field(names::EXP);
    if facts.int(&exp)? != first {
        moved(format!("{}.currentExp", facts.char_path), exp, first);
    }
    let id = facts.char_id;
    for ((field, number), (_, old_number)) in after.fields(id).into_iter().zip(before.fields(id)) {
        if number == old_number {
            continue;
        }
        let scalar = KEYS.iter().find(|(name, _)| facts.field(name) == field).map(|(_, key)| *key);
        moved(path_of(facts, &field, scalar)?, field, number);
    }
    Some(Case {
        name: "level_up",
        edits: vec![Edit::set(level_field, Value::Int(i64::from(new)))],
        proof: Proof { changes, view, texts },
        linked: None,
        raw: false,
        rolled: true,
    })
}
