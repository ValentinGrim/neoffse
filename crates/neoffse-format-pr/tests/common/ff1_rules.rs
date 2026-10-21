//! The game's rules for an FF I slot, checked through the public model.
//!
//! A message carries the field path and never a value: the callers also run
//! it on private saves.

use std::collections::HashMap;

use neoffse_core::{FieldId, Value};
use neoffse_format_pr::ff1::{Ff1Model, names};

fn int(model: &mut Ff1Model, field: &FieldId) -> Result<i64, String> {
    match model.get(field) {
        Ok(Value::Int(value)) => Ok(value),
        Ok(_) => Err(format!("{field}: not an integer")),
        Err(error) => Err(format!("{field}: {error}")),
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
/// `copy_counts`), `cap:` for a capped field above its cap.
pub(crate) fn violations(model: &mut Ff1Model, copy_counts: bool) -> Vec<String> {
    let mut found = Vec::new();
    let mut note = |kind: &str, text: String| found.push(format!("{kind}: {text}"));
    let inventory_id = FieldId::new(names::INVENTORY);
    let keys = match model.rows(&inventory_id) {
        Ok(keys) => keys,
        Err(error) => return vec![format!("read: {inventory_id}: {error}")],
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
                    worn.push((field, content, count));
                }
                (first, second) => {
                    for text in [first.err(), second.err()].into_iter().flatten() {
                        note("read", text);
                    }
                }
            }
        }
    }
    for (field, content, count) in worn {
        match counts.get(&content) {
            None => note("inventory", format!("{field} has no inventory row")),
            Some(owned) if *owned < wearers[&content] => note("inventory", format!("{field} is worn more often than owned")),
            Some(owned) if copy_counts && *owned != count => note("copy", format!("{field}.count differs from the inventory")),
            Some(_) => {}
        }
    }
    found.extend(cap_violations(model));
    found
}

/// Capped fields above their cap, as `cap:` messages.
pub(crate) fn cap_violations(model: &mut Ff1Model) -> Vec<String> {
    let mut fields = vec![FieldId::new(names::GIL)];
    let characters = model.rows(&FieldId::new(names::CHARACTER)).unwrap_or_default();
    for id in characters {
        fields.extend(names::character_fields(id));
    }
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        let collection = FieldId::new(list);
        for key in model.rows(&collection).unwrap_or_default() {
            fields.push(collection.clone().with_id(key).with_name(names::COUNT));
        }
    }
    let mut found = Vec::new();
    for field in fields {
        let (Some(meta), Ok(Value::Int(value))) = (model.field_meta(&field), model.get(&field)) else {
            continue;
        };
        if meta.cap.is_some_and(|cap| value > cap) {
            found.push(format!("cap: {field} is above its cap"));
        }
    }
    found
}

/// Asserts that `model` obeys the game's rules; see [`violations`].
pub(crate) fn assert_game_rules(model: &mut Ff1Model, copy_counts: bool) {
    let found = violations(model, copy_counts);
    assert!(found.is_empty(), "{}", found.join("; "));
}
