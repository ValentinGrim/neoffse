//! The game's rules for an FF II slot, checked through the public model.
//!
//! A message carries the field path and never a value or an error text: the
//! callers can also run it on private saves.

use std::collections::HashMap;

use neoffse_core::{FieldId, Value};
use neoffse_data::{BaseStats, Language};
use neoffse_format_pr::document::{Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Model, GAME, names};

use super::character_path;

fn int(model: &mut Ff2Model, field: &FieldId) -> Result<i64, String> {
    match model.get(field) {
        Ok(Value::Int(value)) => Ok(value),
        Ok(_) => Err(format!("{field}: not an integer")),
        Err(_) => Err(format!("{field}: does not read")),
    }
}

/// True when the tables list `content` as a placeholder row (47, 208 to 211).
fn is_placeholder(content: i64) -> bool {
    let tables = neoffse_data::tables_for(GAME);
    let entry = u32::try_from(content).ok().and_then(|id| tables?.content(id));
    entry.is_some_and(|entry| entry.placeholder)
}

fn slot_field(id: u32, slot: u32) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id).with_name(names::EQUIPMENT).with_id(slot)
}

/// Every breach of the rules, each message starting with its kind:
/// `read:` for a field that fails to read, `inventory:` for an equipped id
/// with no row or a count below its wearers
/// (`docs/formats/pixel-remaster.md#inventory`), `copy:` for a slot `count`
/// that differs from the inventory count
/// (`docs/formats/pixel-remaster.md#equipment-slots`, checked when
/// `copy_counts`), `hand:` for the shield placeholder in a hand
/// (`docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`), `cap:` for a
/// capped field above its cap (see [`cap_violations`]).
pub(crate) fn violations(model: &mut Ff2Model, copy_counts: bool) -> Vec<String> {
    let mut found = Vec::new();
    let mut note = |kind: &str, text: String| found.push(format!("{kind}: {text}"));
    let inventory_id = FieldId::new(names::INVENTORY);
    let Ok(keys) = model.rows(&inventory_id) else {
        return vec![format!("read: {inventory_id}: rows do not read")];
    };
    let mut counts = HashMap::new();
    for key in keys {
        let field = inventory_id.clone().with_id(key).with_name(names::COUNT);
        match int(model, &field) {
            Ok(count) => {
                counts.insert(i64::from(key), count);
            }
            Err(text) => note("read", text),
        }
    }
    let Ok(characters) = model.rows(&FieldId::new(names::CHARACTER)) else {
        note("read", names::CHARACTER.to_owned());
        return found;
    };
    let mut wearers: HashMap<i64, i64> = HashMap::new();
    let mut worn = Vec::new();
    for id in characters {
        for slot in 1..=names::EQUIPMENT_SLOTS {
            let field = slot_field(id, slot);
            match (int(model, &field), int(model, &field.clone().with_name(names::COUNT))) {
                (Ok(content), Ok(count)) => {
                    *wearers.entry(content).or_default() += 1;
                    worn.push((field, slot, content, count));
                }
                (first, second) => {
                    for text in [first.err(), second.err()].into_iter().flatten() {
                        note("read", text);
                    }
                }
            }
        }
    }
    for (field, slot, content, count) in worn {
        if slot <= 2 && content == 208 {
            note("hand", format!("{field} holds the shield placeholder"));
        }
        match counts.get(&content) {
            None => note("inventory", format!("{field} has no inventory row")),
            // Placeholder counts do not follow the wearers: docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii
            Some(owned) if *owned < wearers[&content] && !is_placeholder(content) => note("inventory", format!("{field} is worn more often than owned")),
            Some(owned) if copy_counts && *owned != count => note("copy", format!("{field}.count differs from the inventory")),
            Some(_) => {}
        }
    }
    found.extend(cap_violations(model));
    found
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
    let found = violations(model, copy_counts);
    assert!(found.is_empty(), "{}", found.join("; "));
}
