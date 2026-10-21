//! More single-field cases of the edit matrix: one `Set` each, to a nudged
//! value (`docs/formats/pixel-remaster.md#characters`). Each reads the old
//! value from the view and the position from the tree, so no value of a
//! private save appears in a message.

use neoffse_format_pr::ff1::names;

use super::exit_cases::{Case, nudged};
use super::exit_facts::Facts;

/// `(case, field, minimum, JSON key below the character)` of the scalar
/// fields.
const SCALARS: [(&str, &str, i64, &str); 5] = [
    ("exp", names::EXP, 0, "currentExp"),
    ("hp", names::HP, 0, "parameter.currentHP"),
    ("max_hp_gain", names::MAX_HP_GAIN, 0, "parameter.addtionalMaxHp"),
    ("luck", names::LUCK, 0, "parameter.addtionalLuck"),
    ("magic", names::MAGIC, 0, "parameter.addtionalMagic"),
];

/// The charges dictionary entry of `level`: the case, the field and the list
/// key of `parameter` that holds it.
fn dictionary(facts: &Facts<'_>, name: &'static str, field: &'static str, list: &str, level: u32) -> Option<Case> {
    let id = facts.field(field).with_id(level);
    facts.view.find(&id)?;
    let list = format!("{}.parameter.{list}", facts.char_path);
    let index = facts.subject.tree.find_int(&format!("{list}.keys"), i64::from(level))?;
    nudged(facts, name, id, 0, format!("{list}.values[{index}]"))
}

/// Every case of this file, skipped ones included as `None`.
pub(super) fn field_cases(facts: &Facts<'_>) -> Vec<(&'static str, Option<Case>)> {
    let mut cases: Vec<(&'static str, Option<Case>)> = SCALARS
        .iter()
        .map(|(name, field, min, key)| {
            let path = format!("{}.{key}", facts.char_path);
            (*name, nudged(facts, name, facts.field(field), *min, path))
        })
        .collect();
    cases.push((
        "charges_l8",
        dictionary(facts, "charges_l8", names::CHARGES, "currentMpCountList", names::SPELL_LEVELS),
    ));
    cases.push((
        "charges_max_gain_l1",
        dictionary(facts, "charges_max_gain_l1", names::CHARGES_MAX_GAIN, "addtionalMaxMpCountList", 1),
    ));
    cases
}
