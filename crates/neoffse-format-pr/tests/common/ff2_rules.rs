//! The game's rules for an FF II slot, checked through the public model: the
//! shared core of `slot_rules` with FF II's placeholder exemption, hand rule
//! and cap list.
//!
//! A message carries the field path and never a value or an error text: the
//! callers can also run it on private saves.

use neoffse_core::{FieldId, Value};
use neoffse_data::{BaseStats, Language};
use neoffse_format_pr::document::{Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Model, GAME, names};

use super::character_path;
use super::slot_rules::{self, Rules};

const RULES: Rules<Ff2Model> = Rules {
    exempt_placeholder: true,
    worn: shield_in_hand,
    caps: cap_violations,
};

/// The shield placeholder (208) in slot 1 or 2, the hands
/// (`docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`).
fn shield_in_hand(field: &FieldId, slot: u32, content: i64) -> Option<String> {
    (slot <= 2 && content == 208).then(|| format!("{field} holds the shield placeholder"))
}

/// Every breach of the rules; see [`slot_rules::violations`]. `hand:` flags
/// the shield placeholder in a hand, `cap:` follows [`cap_violations`].
pub(crate) fn violations(model: &mut Ff2Model, copy_counts: bool) -> Vec<String> {
    slot_rules::violations(model, copy_counts, &RULES)
}

/// Each gain with its base in the status row and the `bounds.toml` key of
/// the cap of base plus gain
/// (`docs/formats/pixel-remaster.md#character-growth-in-ff-ii`).
type Gain = (&'static str, fn(&BaseStats) -> u32, &'static str);
const GAINS: [Gain; 8] = [
    (names::MAX_HP_GAIN, |base| base.hp, "MAX_HP"),
    (names::MAX_MP_GAIN, |base| base.mp, "MAX_MP"),
    (names::POWER, |base| base.power, "MAX_STRNGTH"),
    (names::VITALITY, |base| base.vitality, "MAX_VITALITY"),
    (names::AGILITY, |base| base.agility, "MAX_AGILITY"),
    (names::INTELLIGENCE, |base| base.intelligence, "MAX_INTELIGENCE"),
    (names::SPIRIT, |base| base.spirit, "MAX_SPRIT"),
    (names::MAGIC, |base| base.magic, "MAX_MAGIC"),
];

/// Capped fields above their cap, as `cap:` messages: every field of the view
/// with a game cap (gil, HP, MP, the skills, the item counts, the proficiency
/// of each stored spell place), and each gain whose base plus gain passes the
/// cap of its stat. The game stops its own raises at the cap, so its saves
/// stay at or under it (`docs/formats/pixel-remaster.md#character-growth-in-ff-ii`,
/// "Reading a stored gain").
pub(crate) fn cap_violations(model: &mut Ff2Model) -> Vec<String> {
    let Ok(view) = model.view(Language::En) else {
        return vec!["read: the view does not build".to_owned()];
    };
    let mut found = Vec::new();
    for field in view.sections.iter().flat_map(|section| &section.entities).flat_map(|entity| &entity.fields) {
        if let (Some(cap), Value::Int(value)) = (field.meta.cap, &field.value)
            && *value > cap
        {
            found.push(format!("cap: {} is above its cap", field.id));
        }
    }
    found.extend(gain_violations(model));
    found
}

/// A gain past `cap - base`, or a status with no base row.
fn gain_violations(model: &mut Ff2Model) -> Vec<String> {
    let Some(tables) = neoffse_data::tables_for(GAME) else {
        return vec!["read: no FF II tables".to_owned()];
    };
    let Ok(ids) = model.rows(&FieldId::new(names::CHARACTER)) else {
        return vec![format!("read: {}: rows do not read", names::CHARACTER)];
    };
    let mut found = Vec::new();
    for (index, id) in ids.into_iter().enumerate() {
        let path = character_path(index, &[Segment::Doc, Segment::Key("characterStatusId")]);
        let status = match model.read(&path) {
            Ok(Scalar::Int(status)) => u32::try_from(status).ok(),
            _ => None,
        };
        let Some(base) = status.and_then(|status| tables.base_stats(status)) else {
            found.push(format!("read: {}.{id} has no base row", names::CHARACTER));
            continue;
        };
        for (name, base_of, key) in GAINS {
            let field = FieldId::new(names::CHARACTER).with_id(id).with_name(name);
            let Ok(Value::Int(gain)) = model.get(&field) else {
                found.push(format!("read: {field} does not read"));
                continue;
            };
            if tables.bound(key).is_some_and(|cap| i64::from(base_of(&base)) + gain > cap) {
                found.push(format!("cap: {field} plus its base is above its cap"));
            }
        }
    }
    found
}

/// Asserts that `model` obeys the game's rules; see [`violations`].
pub(crate) fn assert_game_rules(model: &mut Ff2Model, copy_counts: bool) {
    slot_rules::assert_rules(model, copy_counts, &RULES);
}
