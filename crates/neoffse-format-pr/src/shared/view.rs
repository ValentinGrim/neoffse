//! The parts of a view that every game builds the same way: the label of a
//! character and the inventory section (gil, item rows, key item rows).

use neoffse_core::{Capability, EditError, Entity, FieldId, FieldView, FormatError, RowId, Section, Value};
use neoffse_data::Language;

use super::SlotModel;
use super::names;
use super::read::decode_error;
use crate::document::DocumentError;

/// True when the save does not hold the field: a dictionary without the key
/// or a plain key that is absent.
pub(crate) fn is_absent(error: &EditError) -> bool {
    match error {
        EditError::UnknownField { .. } => true,
        EditError::Format { source, .. } => matches!(source.downcast_ref::<DocumentError>(), Some(DocumentError::MissingKey { .. })),
        _ => false,
    }
}

/// The name of character `id` when `fields` holds a non-empty one, else
/// `Character <id>`.
pub(crate) fn character_label(id: u32, fields: &[FieldView]) -> String {
    let name_id = FieldId::new(names::CHARACTER).with_id(id).with_name(names::NAME);
    fields
        .iter()
        .find_map(|view| match &view.value {
            Value::Text(name) if view.id == name_id && !name.is_empty() => Some(name.clone()),
            _ => None,
        })
        .unwrap_or_else(|| format!("Character {id}"))
}

/// The view of `id`, `None` when the save does not hold the field.
pub(crate) fn field_view<M: SlotModel>(model: &mut M, id: FieldId) -> Result<Option<FieldView>, FormatError> {
    let Some(meta) = model.field_meta(&id) else {
        return Err(decode_error(EditError::UnknownField { field: id }));
    };
    match model.get(&id) {
        Ok(value) => Ok(Some(FieldView::new(id, meta, value))),
        Err(error) if is_absent(&error) => Ok(None),
        Err(error) => Err(decode_error(error)),
    }
}

/// The row keys of `collection`, the error of the read as a decode error.
pub(crate) fn rows_of<M: SlotModel>(model: &mut M, collection: &FieldId) -> Result<Vec<u32>, FormatError> {
    model.rows(collection).map_err(decode_error)
}

/// The Inventory section: the gil entity when `owendGil` is present, then the
/// rows of `inventory` and of `key_items`. Only the item names follow
/// `language`; an item the tables do not name is labelled `Item <id>` or
/// `Key item <id>`.
pub(crate) fn inventory_section<M: SlotModel>(model: &mut M, language: Language) -> Result<Section, FormatError> {
    let mut entities = Vec::new();
    let gil = FieldId::new(names::GIL);
    if let Some(view) = field_view(model, gil.clone())? {
        entities.push(Entity::new(gil, "Gil", vec![view]));
    }
    let lists = [(names::INVENTORY, "Item"), (names::KEY_ITEMS, "Key item")];
    for (list, fallback) in lists {
        let collection = FieldId::new(list);
        for key in rows_of(model, &collection)? {
            let row = RowId::new(collection.clone(), key);
            let label = model
                .tables()
                .content(key)
                .and_then(|entry| entry.name(language))
                .filter(|name| !name.is_empty())
                .map_or_else(|| format!("{fallback} {key}"), str::to_owned);
            let fields = field_view(model, row.field(names::COUNT))?.into_iter().collect();
            entities.push(Entity::new(row.id().clone(), label, fields));
        }
    }
    let mut section = Section::new(Capability::Inventory, entities);
    for list in [names::INVENTORY, names::KEY_ITEMS] {
        section.collections.extend(model.collection_meta(&FieldId::new(list)));
    }
    Ok(section)
}
