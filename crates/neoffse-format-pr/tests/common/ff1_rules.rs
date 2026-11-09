//! The game's rules for an FF I slot, checked through the public model: the
//! shared core of `slot_rules` with FF I's cap list.
//!
//! A message carries the field path and never a value or an error text: the
//! callers also run it on private saves.

use neoffse_format_pr::ff1::Ff1Model;

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

/// Capped fields above their cap, as `cap:` messages: [`slot_rules::view_caps`].
pub(crate) fn cap_violations(model: &mut Ff1Model) -> Vec<String> {
    slot_rules::view_caps(model)
}

/// Asserts that `model` obeys the game's rules; see [`violations`].
pub(crate) fn assert_game_rules(model: &mut Ff1Model, copy_counts: bool) {
    slot_rules::assert_rules(model, copy_counts, &RULES);
}
