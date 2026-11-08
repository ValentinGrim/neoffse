//! Helpers of the private-save tests, for every game. A failure message
//! carries the slot label and a field path, never a value, a length or a name
//! (`docs/architecture.md#test-policy`).

use std::collections::BTreeMap;

use neoffse_core::{Edit, EditError, Entity, FieldId, Value};
use neoffse_data::GameTables;
use neoffse_format_pr::shared::{SlotModel, names};

use super::edit_subject::Subject;
use super::exit_facts::{CHARS, INV};
use super::ff1_real::{RealSlot, must, must_some};

/// The tables of the game of `M`.
pub(crate) fn tables<M: SlotModel>() -> &'static GameTables {
    must_some(neoffse_data::tables_for(M::GAME), &format!("no {:?} tables", M::GAME))
}

/// The row keys of `list`.
pub(crate) fn rows<M: SlotModel>(model: &mut M, list: &'static str, label: &str) -> Vec<u32> {
    must(model.rows(&FieldId::new(list)), &format!("{label}: {list} rows do not read"))
}

/// The integer at `field`; the message names the field and holds no value.
pub(crate) fn int<M: SlotModel>(model: &mut M, label: &str, field: &FieldId) -> i64 {
    match model.get(field) {
        Ok(Value::Int(value)) => value,
        _ => panic!("{label}: {field} does not read as an integer"),
    }
}

/// `gil`, the fields `character_fields` lists for every character, and every
/// item count of both lists.
pub(crate) fn all_fields<M: SlotModel>(model: &mut M, label: &str, character_fields: fn(u32) -> Vec<FieldId>) -> Vec<FieldId> {
    let mut fields = vec![FieldId::new(names::GIL)];
    for id in rows(model, names::CHARACTER, label) {
        fields.extend(character_fields(id));
    }
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        let collection = FieldId::new(list);
        fields.extend(
            rows(model, list, label)
                .into_iter()
                .map(|key| collection.clone().with_id(key).with_name(names::COUNT)),
        );
    }
    fields
}

/// Runs `check` on a model of each of `slots`; no slots (the skip is already
/// reported) runs nothing.
pub(crate) fn for_each_slot<M: SlotModel>(slots: Option<Vec<RealSlot>>, mut check: impl FnMut(&RealSlot, &mut M)) {
    let Some(slots) = slots else { return };
    for slot in slots {
        let mut model = slot.model::<M>();
        check(&slot, &mut model);
    }
}

/// Every field of `entities` reads from `model` as the view shows it.
pub(crate) fn assert_fields_read_back<'a, M: SlotModel>(label: &str, model: &mut M, entities: impl IntoIterator<Item = &'a Entity>) {
    for entity in entities {
        for field in &entity.fields {
            let Ok(value) = model.get(&field.id) else {
                panic!("{label}: {} does not read", field.id)
            };
            assert!(value == field.value, "{label}: {} differs from the model", field.id);
        }
    }
}

/// The skipped cases with their counts, or `none`.
pub(crate) fn skipped_line(skipped: &BTreeMap<&'static str, usize>) -> String {
    if skipped.is_empty() {
        return "none".to_owned();
    }
    skipped.iter().map(|(name, count)| format!("{name} x{count}")).collect::<Vec<_>>().join(", ")
}

/// True when the tables of `M` list `content` as a placeholder row.
pub(crate) fn is_placeholder<M: SlotModel>(content: impl Into<i64>) -> bool {
    let tables = neoffse_data::tables_for(M::GAME);
    let entry = u32::try_from(content.into()).ok().and_then(|id| tables?.content(id));
    entry.is_some_and(|entry| entry.placeholder)
}

/// Every content id that a character wears, in document order.
pub(crate) fn worn_ids(subject: &Subject) -> Vec<u32> {
    let mut ids = Vec::new();
    for index in 0..subject.tree.array_len(CHARS).unwrap_or(0) {
        let list = format!("{CHARS}[{index}].equipmentList.values");
        for slot in 0..subject.tree.array_len(&list).unwrap_or(0) {
            ids.extend(subject.tree.int(&format!("{list}[{slot}].contentId")).and_then(|id| u32::try_from(id).ok()));
        }
    }
    ids
}

/// The inventory ids in document order.
pub(crate) fn inventory_ids(subject: &Subject) -> Vec<u32> {
    (0..subject.tree.array_len(INV).unwrap_or(0))
        .filter_map(|index| subject.tree.int(&format!("{INV}[{index}].contentId")))
        .filter_map(|id| u32::try_from(id).ok())
        .collect()
}

/// True for the refusal of a read-only field or row.
pub(crate) fn is_read_only(error: &EditError) -> bool {
    matches!(error, EditError::ReadOnly { .. })
}

/// `edit` meets an error that `matches` accepts, and the document still
/// encodes to the subject's text.
pub(crate) fn refuses(subject: &Subject, what: &str, edit: &Edit, matches: impl Fn(&EditError) -> bool) {
    let mut document = subject.open();
    let result = document.apply_game(edit);
    assert!(result.as_ref().err().is_some_and(matches), "{} {what}: not refused as expected", subject.label);
    subject.assert_original(&*document, what);
}
