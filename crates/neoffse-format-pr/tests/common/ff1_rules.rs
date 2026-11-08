//! The game's rules for an FF I slot, checked through the public model: the
//! shared core of `slot_rules` with FF I's cap list.
//!
//! A message carries the field path and never a value or an error text: the
//! callers also run it on private saves.

use neoffse_core::{FieldId, Value};
use neoffse_format_pr::ff1::{Ff1Model, names};

use super::slot_rules::{self, Rules};

const RULES: Rules<Ff1Model> = Rules {
    exempt_placeholder: false,
    worn: |_, _, _| None,
    caps: cap_violations,
};

/// Every breach of the rules; see [`slot_rules::violations`]. FF I exempts no
/// placeholder row and has no hand rule.
pub(crate) fn violations(model: &mut Ff1Model, copy_counts: bool) -> Vec<String> {
    slot_rules::violations(model, copy_counts, &RULES)
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
    slot_rules::assert_rules(model, copy_counts, &RULES);
}
