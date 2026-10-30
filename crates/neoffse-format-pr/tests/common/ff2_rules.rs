//! The game's rules for an FF II slot, checked through the public model.
//!
//! A message carries the field path and never a value or an error text: the
//! callers can also run it on private saves.

use std::collections::HashMap;

use neoffse_core::{FieldId, Value};
use neoffse_format_pr::ff2::{Ff2Model, names};

fn int(model: &mut Ff2Model, field: &FieldId) -> Result<i64, String> {
    match model.get(field) {
        Ok(Value::Int(value)) => Ok(value),
        Ok(_) => Err(format!("{field}: not an integer")),
        Err(_) => Err(format!("{field}: does not read")),
    }
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
/// (`docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`).
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
            Some(owned) if *owned < wearers[&content] => note("inventory", format!("{field} is worn more often than owned")),
            Some(owned) if copy_counts && *owned != count => note("copy", format!("{field}.count differs from the inventory")),
            Some(_) => {}
        }
    }
    found
}

/// Asserts that `model` obeys the game's rules; see [`violations`].
pub(crate) fn assert_game_rules(model: &mut Ff2Model, copy_counts: bool) {
    let found = violations(model, copy_counts);
    assert!(found.is_empty(), "{}", found.join("; "));
}
