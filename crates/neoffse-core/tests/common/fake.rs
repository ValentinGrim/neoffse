//! A fake save format driving every trait test: scalar fields with metadata,
//! an ordered collection keyed by `u32`, a hidden text the view never shows,
//! and a strict text encoding in one file.
//! The text codec lives in `fake_codec.rs`.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::collections::BTreeMap;
use std::time::Duration;

use neoffse_core::{
    CapPolicy, Capability, CollectionMeta, Confidence, Detection, Edit, EditError, Entity, FieldId, FieldView, FileProbe, FormatError, Game, Key, RowId,
    SaveDocument, SaveFiles, SaveFormat, SaveSlot, SaveSummary, SaveView, Section, Value,
};

use super::fake_codec::{self, FakeError};
pub(crate) use super::fake_meta::*;

/// The one file of the fake format.
pub(crate) const FILE: &str = "fake.sav";
/// The first line of the file.
pub(crate) const HEADER: &str = "NEOFFSE-FAKE 1";

/// The format of the fake: a unit struct.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FakeFormat;

#[derive(Debug, Clone)]
pub(crate) struct FakeDocument {
    pub(crate) hidden: String,
    pub(crate) scalars: BTreeMap<FieldId, Slot>,
    pub(crate) rows: Vec<(u32, i64)>,
}

impl FakeDocument {
    pub(crate) fn sample() -> Self {
        Self {
            hidden: String::new(),
            scalars: scalar_slots(),
            rows: vec![(2, 3), (3, 1)],
        }
    }

    /// The scalar field ids of the fake.
    pub(crate) fn scalar_ids() -> Vec<FieldId> {
        scalar_slots().into_keys().collect()
    }

    /// The text only `encode` and `verify` see.
    pub(crate) fn set_hidden(&mut self, text: &str) {
        self.hidden = text.to_string();
    }

    pub(crate) fn boxed() -> Box<dyn SaveDocument> {
        Box::new(Self::sample())
    }

    /// The files `encode` writes for the sample.
    pub(crate) fn sample_files() -> SaveFiles {
        Self::sample().encode().unwrap()
    }

    /// Whether `other` holds the same scalar values, hidden text and rows.
    fn same_as(&self, other: &FakeDocument) -> bool {
        let values = |doc: &FakeDocument| doc.scalars.iter().map(|(id, s)| (id.clone(), s.value.clone())).collect::<Vec<_>>();
        self.hidden == other.hidden && self.rows == other.rows && values(self) == values(other)
    }

    fn index_of(&self, key: u32) -> Option<usize> {
        self.rows.iter().position(|(k, _)| *k == key)
    }

    fn collection_meta(id: &FieldId) -> Option<CollectionMeta> {
        [inventory_meta(), character_meta()].into_iter().find(|meta| meta.id == *id)
    }

    fn text_of(&self, id: &FieldId) -> String {
        match self.scalars.get(id).map(|slot| &slot.value) {
            Some(Value::Text(text)) => text.clone(),
            _ => String::new(),
        }
    }

    fn set(&mut self, field: &FieldId, value: &Value, policy: CapPolicy) -> Result<Edit, EditError> {
        if let [Key::Name("inventory"), Key::Id(key), Key::Name(leaf)] = field.keys() {
            let Some(index) = self.index_of(*key) else {
                return Err(EditError::MissingRow { row: row(*key).id().clone() });
            };
            if *leaf != "count" {
                return Err(EditError::UnknownField { field: field.clone() });
            }
            count_field().meta.check(field, value, policy)?;
            let (Value::Int(count), Some(entry)) = (value, self.rows.get_mut(index)) else {
                return Err(EditError::UnknownField { field: field.clone() });
            };
            let old = std::mem::replace(&mut entry.1, *count);
            return Ok(Edit::set(field.clone(), Value::Int(old)));
        }
        let Some(entry) = self.scalars.get_mut(field) else {
            return Err(EditError::UnknownField { field: field.clone() });
        };
        entry.meta.check(field, value, policy)?;
        let old = std::mem::replace(&mut entry.value, value.clone());
        Ok(Edit::set(field.clone(), old))
    }

    fn insert(&mut self, id: &RowId, position: Option<usize>, values: &[(&'static str, Value)], policy: CapPolicy) -> Result<Edit, EditError> {
        let collection = id.collection();
        let Some(meta) = Self::collection_meta(&collection) else {
            return Err(EditError::NotACollection { field: collection });
        };
        meta.check_insert(id, values, policy)?;
        if collection != inventory() {
            return Err(EditError::NotACollection { field: collection });
        }
        if !ITEMS.contains(&id.key()) {
            return Err(EditError::UnknownId {
                field: id.id().clone(),
                id: i64::from(id.key()),
            });
        }
        if self.index_of(id.key()).is_some() {
            return Err(EditError::DuplicateRow { row: id.id().clone() });
        }
        let len = self.rows.len();
        let position = position.unwrap_or(len);
        if position > len {
            return Err(EditError::PositionOutOfRange { collection, position, len });
        }
        let count = values
            .iter()
            .find_map(|(_, value)| if let Value::Int(n) = value { Some(*n) } else { None })
            .unwrap_or(1);
        self.rows.insert(position, (id.key(), count));
        Ok(Edit::remove(id.clone()))
    }

    fn remove(&mut self, id: &RowId) -> Result<Edit, EditError> {
        let collection = id.collection();
        let Some(meta) = Self::collection_meta(&collection) else {
            return Err(EditError::NotACollection { field: collection });
        };
        meta.check_remove(id)?;
        let Some(index) = self.index_of(id.key()) else {
            return Err(EditError::MissingRow { row: id.id().clone() });
        };
        let (_, count) = self.rows.remove(index);
        Ok(Edit::insert(id.clone(), Some(index), vec![("count", Value::Int(count))]))
    }

    fn build_view(&self) -> SaveView {
        let field_view = |id: &FieldId, slot: &Slot| FieldView::new(id.clone(), slot.meta.clone(), slot.value.clone());
        let party: Vec<Entity> = [1, 2]
            .into_iter()
            .map(|n| {
                let prefix = character().with_id(n);
                let fields = self
                    .scalars
                    .iter()
                    .filter(|(id, _)| id.starts_with(&prefix))
                    .map(|(id, slot)| field_view(id, slot))
                    .collect();
                Entity::new(prefix, format!("Character {n}"), fields)
            })
            .collect();
        let mut party_section = Section::new(Capability::Party, party);
        party_section.collections.push(character_meta());

        let mut entities: Vec<Entity> = self
            .scalars
            .iter()
            .filter(|(id, _)| **id == gil())
            .map(|(id, slot)| Entity::new(id.clone(), "Gil", vec![field_view(id, slot)]))
            .collect();
        for (key, count) in &self.rows {
            let id = row(*key);
            let field = FieldView::new(id.field("count"), count_field().meta, Value::Int(*count));
            entities.push(Entity::new(id.id().clone(), format!("Item {key}"), vec![field]));
        }
        let mut inventory_section = Section::new(Capability::Inventory, entities);
        inventory_section.collections.push(inventory_meta());
        SaveView::new(vec![party_section, inventory_section])
    }
}

pub(crate) fn decode_error(error: FakeError) -> FormatError {
    FormatError::Decode { source: Box::new(error) }
}

fn decode_files(files: &SaveFiles) -> Result<FakeDocument, FormatError> {
    let bytes = files.get(FILE).ok_or_else(|| FormatError::MissingFile { name: FILE.into() })?;
    fake_codec::decode(bytes).map_err(decode_error)
}

impl SaveFormat for FakeFormat {
    fn game(&self) -> Game {
        Game::Ff1Pr
    }

    fn detect(&self, probe: &FileProbe<'_>) -> Option<Detection> {
        if probe.name != FILE {
            return None;
        }
        let confidence = if probe.head.starts_with(HEADER.as_bytes()) {
            Confidence::Certain
        } else {
            Confidence::Likely
        };
        let mut detection = Detection::new(confidence);
        detection.slot = Some(SaveSlot::Numbered(1));
        Some(detection)
    }

    fn open(&self, files: &SaveFiles) -> Result<Box<dyn SaveDocument>, FormatError> {
        Ok(Box::new(decode_files(files)?))
    }
}

impl SaveDocument for FakeDocument {
    fn game(&self) -> Game {
        Game::Ff1Pr
    }

    fn capabilities(&self) -> &[Capability] {
        &[Capability::Party, Capability::Inventory]
    }

    fn summary(&mut self) -> Result<SaveSummary, FormatError> {
        let mut summary = SaveSummary::new(Game::Ff1Pr);
        summary.slot = Some(SaveSlot::Numbered(1));
        summary.play_time = Some(Duration::from_secs(3_600));
        summary.location = Some("Lakeside".into());
        summary.party = vec![self.text_of(&name(1)), self.text_of(&name(2))];
        Ok(summary)
    }

    fn view(&mut self) -> Result<SaveView, FormatError> {
        Ok(self.build_view())
    }

    fn apply(&mut self, edit: &Edit, policy: CapPolicy) -> Result<Edit, EditError> {
        match edit {
            Edit::Set { field, value } => self.set(field, value, policy),
            Edit::Insert { row, position, values } => self.insert(row, *position, values, policy),
            Edit::Remove { row } => self.remove(row),
            _ => Err(EditError::UnknownField { field: edit.target().clone() }),
        }
    }

    fn encode(&self) -> Result<SaveFiles, FormatError> {
        let mut files = SaveFiles::new();
        files.push(FILE, fake_codec::encode(self).into_bytes())?;
        Ok(files)
    }

    fn verify(&self, files: &SaveFiles) -> Result<(), FormatError> {
        let decoded = decode_files(files)?;
        if self.same_as(&decoded) {
            Ok(())
        } else {
            Err(FormatError::VerifyMismatch { name: FILE.into() })
        }
    }
}

pub(crate) fn open(files: &SaveFiles) -> Box<dyn SaveDocument> {
    let format: &dyn SaveFormat = &FakeFormat;
    format.open(files).unwrap()
}

pub(crate) fn doc() -> Box<dyn SaveDocument> {
    FakeDocument::boxed()
}

pub(crate) fn bytes(doc: &dyn SaveDocument) -> Vec<u8> {
    doc.encode().unwrap().get(FILE).unwrap().to_vec()
}

pub(crate) fn files_of(bytes: Vec<u8>) -> SaveFiles {
    let mut files = SaveFiles::new();
    files.push(FILE, bytes).unwrap();
    files
}

pub(crate) fn add(key: u32, position: Option<usize>, count: i64) -> Edit {
    Edit::insert(row(key), position, vec![("count", Value::Int(count))])
}
