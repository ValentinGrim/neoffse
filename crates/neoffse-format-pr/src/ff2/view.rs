//! The view of a slot: the party and the inventory as the editor shows them.

use neoffse_core::{Capability, Entity, FieldId, FormatError, RowId, SaveView, Section, TableKind};
use neoffse_data::Language;

use super::{Ff2Model, names, places, spell};
use crate::shared::read::decode_error;
use crate::shared::view::{character_label, field_view, inventory_section, row_label, rows_of};

impl Ff2Model {
    /// A snapshot of the slot: a Party section with one entity per character
    /// in document order (34 fields each, `names::party_fields`: the 32
    /// stored fields plus the computed `max_hp` and `max_mp`), then an
    /// Inventory section with the gil entity, the rows of `inventory` and the
    /// rows of `key_items` and the keywords of `ownedKeyWaordList` (named by
    /// the `Keyword` table, `Entry <id>` when it holds no name, `Keyword <id>`
    /// when it lists none), then a Magic section.
    ///
    /// The Magic section holds one entity per character in the same order,
    /// labelled like the Party entity, with one Level field per owned spell
    /// in `abilityList` order. The label of a Level field is the spell's name
    /// in `language`, `Entry <id>` of the chain's first ability when the row
    /// has none. Ability 711 and any other entry outside the 40 chains have
    /// no field (`docs/formats/pixel-remaster.md#spells-in-ff-ii`).
    ///
    /// After the Level entities, the section holds one raw page per character
    /// that has a stored place, id `character.<id>.raw_spells` and label
    /// `<name>, raw spells`: the `abilityList` entries (three fields each),
    /// the dictionary entries (three each), the addition order, the sort
    /// order. Each label names the entry by its current ability, for example
    /// `List 1, Entry 711, row` or `Copy 2, Fire, value`. A character holds
    /// up to 8 fields per spell plus 4, so 132 for 16 spells.
    ///
    /// Only the item names of the row entities and the spell labels follow
    /// `language`; the other field labels stay English. A row the tables list without a name is labelled
    /// `Entry <id>`, and an item the tables do not list `Item <id>` or
    /// `Key item <id>`. A field the save does not hold is left out of its
    /// entity, and the gil entity is left out when `owendGil` is absent. The
    /// call changes no byte of the document.
    ///
    /// # Errors
    ///
    /// `FormatError::Decode` with the `EditError` source for any other
    /// failure, such as a float where an integer belongs.
    pub fn view(&mut self, language: Language) -> Result<SaveView, FormatError> {
        let party = self.party_section()?;
        let inventory = self.inventory_with_keywords(language)?;
        let magic = self.magic_section(language)?;
        Ok(SaveView::new(vec![party, inventory, magic]))
    }

    /// The shared Inventory section, then one entity per keyword and the
    /// `keywords` collection meta. A keyword row holds no field.
    fn inventory_with_keywords(&mut self, language: Language) -> Result<Section, FormatError> {
        let mut section = inventory_section(self, language)?;
        let collection = FieldId::new(names::KEYWORDS);
        for key in rows_of(self, &collection)? {
            let row = RowId::new(collection.clone(), key);
            let label = row_label(self.tables.entry(TableKind::Keyword, key), key, "Keyword", language);
            section.entities.push(Entity::new(row.id().clone(), label, Vec::new()));
        }
        section.collections.extend(self.collection_meta(&collection));
        Ok(section)
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

    fn magic_section(&mut self, language: Language) -> Result<Section, FormatError> {
        let character = FieldId::new(names::CHARACTER);
        let mut entities = Vec::new();
        let mut raw = Vec::new();
        for id in rows_of(self, &character)? {
            let entity_id = character.clone().with_id(id);
            let name = field_view(self, entity_id.clone().with_name(names::NAME))?;
            let label = character_label(id, name.as_slice());
            let fields = spell::fields(&mut self.document, self.tables, id, language).map_err(decode_error)?;
            let places = places::fields(&mut self.document, self.tables, id, language).map_err(decode_error)?;
            // An entity with no place is left out.
            if !places.is_empty() {
                raw.push(Entity::new(
                    entity_id.clone().with_name(names::RAW_SPELLS),
                    format!("{label}, raw spells"),
                    places,
                ));
            }
            entities.push(Entity::new(entity_id, label, fields));
        }
        entities.extend(raw);
        Ok(Section::new(Capability::Magic, entities))
    }
}
