//! The view of a slot: the party, the inventory and the magic as the editor
//! shows them.

use neoffse_core::{Capability, EditError, Entity, FieldId, FieldView, FormatError, RowId, SaveView, Section, Value};
use neoffse_data::Language;

use super::summary::decode_error;
use super::{Ff1Model, names};
use crate::document::DocumentError;

/// True when the save does not hold the field: a dictionary without the key
/// or a plain key that is absent.
fn is_absent(error: &EditError) -> bool {
    match error {
        EditError::UnknownField { .. } => true,
        EditError::Format { source, .. } => matches!(source.downcast_ref::<DocumentError>(), Some(DocumentError::MissingKey { .. })),
        _ => false,
    }
}

/// The name of character `id` when `fields` holds a non-empty one, else
/// `Character <id>`.
fn character_label(id: u32, fields: &[FieldView]) -> String {
    let name_id = FieldId::new(names::CHARACTER).with_id(id).with_name(names::NAME);
    fields
        .iter()
        .find_map(|view| match &view.value {
            Value::Text(name) if view.id == name_id && !name.is_empty() => Some(name.clone()),
            _ => None,
        })
        .unwrap_or_else(|| format!("Character {id}"))
}

impl Ff1Model {
    /// A snapshot of the slot: a Party section with one entity per character
    /// in document order (43 fields each, `names::party_fields`: the 41 stored
    /// fields plus the computed `max_hp` and `next_level`), an Inventory section with the gil entity, the rows
    /// of `inventory` and the rows of `key_items`, then a Magic section with
    /// one entity per character in the same order. A Magic entity holds the
    /// read-only spell slots of its character (`names::spell_fields`) and its
    /// label follows the Party rule.
    ///
    /// Only the item names of the row entities follow `language`; the field
    /// labels stay English. An item the tables do not name is labelled
    /// `Item <id>` or `Key item <id>`. A field the save does not hold is left
    /// out of its entity, and the gil entity is left out when `owendGil` is
    /// absent. The call changes no byte of the document.
    ///
    /// # Errors
    ///
    /// `FormatError::Decode` with the `EditError` source for any other
    /// failure, such as a float where an integer belongs.
    pub fn view(&mut self, language: Language) -> Result<SaveView, FormatError> {
        let party = self.party_section()?;
        let inventory = self.inventory_section(language)?;
        let magic = self.magic_section()?;
        Ok(SaveView::new(vec![party, inventory, magic]))
    }

    /// The view of `id`, `None` when the save does not hold the field.
    fn field_view(&mut self, id: FieldId) -> Result<Option<FieldView>, FormatError> {
        let Some(meta) = self.field_meta(&id) else {
            return Err(decode_error(EditError::UnknownField { field: id }));
        };
        match self.get(&id) {
            Ok(value) => Ok(Some(FieldView::new(id, meta, value))),
            Err(error) if is_absent(&error) => Ok(None),
            Err(error) => Err(decode_error(error)),
        }
    }

    fn rows_of(&mut self, collection: &FieldId) -> Result<Vec<u32>, FormatError> {
        self.rows(collection).map_err(decode_error)
    }

    fn party_section(&mut self) -> Result<Section, FormatError> {
        let character = FieldId::new(names::CHARACTER);
        let mut entities = Vec::new();
        for id in self.rows_of(&character)? {
            let entity_id = character.clone().with_id(id);
            let mut fields = Vec::new();
            for field in names::party_fields(id) {
                fields.extend(self.field_view(field)?);
            }
            let label = character_label(id, &fields);
            entities.push(Entity::new(entity_id, label, fields));
        }
        let mut section = Section::new(Capability::Party, entities);
        section.collections = self.collection_meta(&character).into_iter().collect();
        Ok(section)
    }

    fn magic_section(&mut self) -> Result<Section, FormatError> {
        let character = FieldId::new(names::CHARACTER);
        let mut entities = Vec::new();
        for id in self.rows_of(&character)? {
            let entity_id = character.clone().with_id(id);
            let name = self.field_view(entity_id.clone().with_name(names::NAME))?;
            let label = character_label(id, name.as_slice());
            let mut fields = Vec::new();
            for field in names::spell_fields(id) {
                fields.extend(self.field_view(field)?);
            }
            entities.push(Entity::new(entity_id, label, fields));
        }
        Ok(Section::new(Capability::Magic, entities))
    }

    fn inventory_section(&mut self, language: Language) -> Result<Section, FormatError> {
        let mut entities = Vec::new();
        let gil = FieldId::new(names::GIL);
        if let Some(view) = self.field_view(gil.clone())? {
            entities.push(Entity::new(gil, "Gil", vec![view]));
        }
        let lists = [(names::INVENTORY, "Item"), (names::KEY_ITEMS, "Key item")];
        for (list, fallback) in lists {
            let collection = FieldId::new(list);
            for key in self.rows_of(&collection)? {
                let row = RowId::new(collection.clone(), key);
                let label = self
                    .tables
                    .content(key)
                    .and_then(|entry| entry.name(language))
                    .filter(|name| !name.is_empty())
                    .map_or_else(|| format!("{fallback} {key}"), str::to_owned);
                let fields = self.field_view(row.field(names::COUNT))?.into_iter().collect();
                entities.push(Entity::new(row.id().clone(), label, fields));
            }
        }
        let mut section = Section::new(Capability::Inventory, entities);
        for list in [names::INVENTORY, names::KEY_ITEMS] {
            section.collections.extend(self.collection_meta(&FieldId::new(list)));
        }
        Ok(section)
    }
}
