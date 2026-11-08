//! The rules of a slot that every Pixel Remaster game shares, checked through
//! the public [`SlotModel`]. A game's wrapper (`ff1_rules`, `ff2_rules`) fills
//! a [`Rules`] with what differs.
//!
//! A message carries the field path and never a value or an error text: the
//! callers can also run it on private saves.

use std::collections::HashMap;

use neoffse_core::{FieldId, Value};
use neoffse_format_pr::shared::SlotModel;
use neoffse_format_pr::shared::names;

use super::private::is_placeholder;

/// What differs between the games.
pub(crate) struct Rules<M> {
    /// True when a placeholder row of the inventory may be worn by more
    /// slots than its count (`docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii`).
    pub(crate) exempt_placeholder: bool,
    /// A `hand:` note for the slot holding `content`, given its field and
    /// slot number: the game's extra rule on one worn slot.
    pub(crate) worn: fn(&FieldId, u32, i64) -> Option<String>,
    /// The `cap:` messages of the game's capped fields.
    pub(crate) caps: fn(&mut M) -> Vec<String>,
}

fn int<M: SlotModel>(model: &mut M, field: &FieldId) -> Result<i64, String> {
    match model.get(field) {
        Ok(Value::Int(value)) => Ok(value),
        Ok(_) => Err(format!("{field}: not an integer")),
        Err(_) => Err(format!("{field}: does not read")),
    }
}

/// Every breach of the rules, each message starting with its kind:
/// `read:` for a field that fails to read, `inventory:` for an equipped id
/// with no row or a count below its wearers
/// (`docs/formats/pixel-remaster.md#inventory`), `copy:` for a slot `count`
/// that differs from the inventory count
/// (`docs/formats/pixel-remaster.md#equipment-slots`, checked when
/// `copy_counts`), `hand:` for a note of `rules.worn`, `cap:` for what
/// `rules.caps` reports.
pub(crate) fn violations<M: SlotModel>(model: &mut M, copy_counts: bool, rules: &Rules<M>) -> Vec<String> {
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
        for number in 1..=names::EQUIPMENT_SLOTS {
            let field = super::slot(id, number);
            match (int(model, &field), int(model, &field.clone().with_name(names::COUNT))) {
                (Ok(content), Ok(count)) => {
                    *wearers.entry(content).or_default() += 1;
                    worn.push((field, number, content, count));
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
        if let Some(text) = (rules.worn)(&field, slot, content) {
            note("hand", text);
        }
        match counts.get(&content) {
            None => note("inventory", format!("{field} has no inventory row")),
            // Placeholder counts do not follow the wearers: docs/formats/pixel-remaster.md#equipment-slots-in-ff-ii
            Some(owned) if *owned < wearers[&content] && !(rules.exempt_placeholder && is_placeholder::<M>(content)) => {
                note("inventory", format!("{field} is worn more often than owned"));
            }
            Some(owned) if copy_counts && *owned != count => note("copy", format!("{field}.count differs from the inventory")),
            Some(_) => {}
        }
    }
    found.extend((rules.caps)(model));
    found
}

/// Asserts that `model` obeys the rules; see [`violations`].
pub(crate) fn assert_rules<M: SlotModel>(model: &mut M, copy_counts: bool, rules: &Rules<M>) {
    let found = violations(model, copy_counts, rules);
    assert!(found.is_empty(), "{}", found.join("; "));
}
