//! Test helpers generic over the slot model: `Ff1Model`, `Ff2Model` and the
//! models of the later games take the same call.

use neoffse_core::{CapPolicy, ChangeSet, FieldId, SaveView};
use neoffse_data::Language;
use neoffse_format_pr::document::Document;
use neoffse_format_pr::shared::{SlotModel, names};

use super::private;
use super::{count, slot, slot_count};

/// A model over `document` with the embedded tables of its game.
pub(crate) fn model_of<M: SlotModel>(document: Document) -> M {
    M::new(document, neoffse_data::tables_for(M::GAME).unwrap())
}

/// `model` with its English view.
pub(crate) fn view_of<M: SlotModel>(mut model: M) -> (M, SaveView) {
    let view = model.view(Language::En).unwrap();
    (model, view)
}

/// The integer at `field` of a synthetic slot; any other value type panics
/// with the field path alone (see [`private::int`]).
pub(crate) fn int<M: SlotModel>(model: &mut M, field: &FieldId) -> i64 {
    private::int(model, "fixture", field)
}

/// Every inventory row with its count, in document order.
pub(crate) fn inventory<M: SlotModel>(model: &mut M) -> Vec<(u32, i64)> {
    let list = FieldId::new(names::INVENTORY);
    model
        .rows(&list)
        .unwrap()
        .into_iter()
        .map(|key| (key, int(model, &count(names::INVENTORY, key))))
        .collect()
}

/// Every slot `contentId` and `count` of every character row, in row order.
pub(crate) fn slots<M: SlotModel>(model: &mut M) -> Vec<(u32, u32, i64, i64)> {
    let mut found = Vec::new();
    for id in model.rows(&FieldId::new(names::CHARACTER)).unwrap() {
        for number in 1..=M::EQUIPMENT_SLOTS {
            found.push((id, number, int(model, &slot(id, number)), int(model, &slot_count(id, number))));
        }
    }
    found
}

/// Replays `set` through the raw interface of `model` under `policy`.
pub(crate) fn run<M: SlotModel>(model: &mut M, set: &ChangeSet, policy: CapPolicy) -> ChangeSet {
    set.apply_with(policy, |edit, policy| model.apply(edit, policy)).unwrap()
}
