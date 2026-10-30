//! The view of a slot: the party, the inventory and the magic as the editor
//! shows them.

use neoffse_core::{Capability, Entity, FieldId, FormatError, SaveView, Section};
use neoffse_data::Language;

use super::{Ff1Model, names};
use crate::shared::view::{character_label, field_view, inventory_section, rows_of};

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
        let inventory = inventory_section(self, language)?;
        let magic = self.magic_section()?;
        Ok(SaveView::new(vec![party, inventory, magic]))
    }

    fn party_section(&mut self) -> Result<Section, FormatError> {
        let character = FieldId::new(names::CHARACTER);
        let mut entities = Vec::new();
        for id in rows_of(self, &character)? {
            let entity_id = character.clone().with_id(id);
            let mut fields = Vec::new();
            for field in names::party_fields(id) {
                fields.extend(field_view(self, field)?);
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
        for id in rows_of(self, &character)? {
            let entity_id = character.clone().with_id(id);
            let name = field_view(self, entity_id.clone().with_name(names::NAME))?;
            let label = character_label(id, name.as_slice());
            let mut fields = Vec::new();
            for field in names::spell_fields(id) {
                fields.extend(field_view(self, field)?);
            }
            entities.push(Entity::new(entity_id, label, fields));
        }
        Ok(Section::new(Capability::Magic, entities))
    }
}
