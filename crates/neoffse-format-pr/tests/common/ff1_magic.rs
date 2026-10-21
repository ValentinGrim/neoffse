//! Helpers of the FF I spell slot tests: the slot rule of
//! `docs/formats/pixel-remaster.md#abilities` ("FF I spell slots") checked
//! through the model, and fixture documents with a twisted
//! `abilitySlotDataList`. A message carries a field path and never a value, so
//! the check can run on private saves.

use std::collections::BTreeSet;

use neoffse_core::{EditError, FieldId, Value};
use neoffse_format_pr::document::{Document, Segment};
use neoffse_format_pr::ff1::{Ff1Model, GAME, names};

use super::embed;
use super::ff1_save::SlotData;
use super::json_tree::Tree;

const CHARS: &str = "userData.ownedCharacterList.target";

/// The ability ids of the spells held by the 24 slots of character `id`.
/// An absent slot counts as empty; a slot the model cannot read is an `Err`
/// with its field path.
pub(crate) fn slotted(model: &mut Ff1Model, id: u32) -> Result<BTreeSet<i64>, String> {
    let mut found = BTreeSet::new();
    for field in names::spell_fields(id) {
        match model.get(&field) {
            Ok(Value::Int(ability)) => {
                if ability != 0 {
                    found.insert(ability);
                }
            }
            Err(EditError::UnknownField { .. }) => {}
            Ok(_) | Err(_) => return Err(format!("{field}: does not read as an ability id")),
        }
    }
    Ok(found)
}

/// The `abilityId`s of `abilityList` of character `id`, read from the tree.
fn listed(tree: &Tree, id: u32) -> Option<BTreeSet<i64>> {
    let base = format!("{CHARS}[{}].abilityList.target", tree.find_row(CHARS, "id", i64::from(id))?);
    (0..tree.array_len(&base)?)
        .map(|index| tree.int(&format!("{base}[{index}].abilityId")))
        .collect()
}

/// One message per character whose slots, as a set without 0, differ from
/// the `abilityId`s of its `abilityList`.
pub(crate) fn slot_mismatches(model: &mut Ff1Model, json: &str) -> Vec<String> {
    let tree = Tree::parse(json);
    let mut found = Vec::new();
    for index in 0..tree.array_len(CHARS).unwrap_or(0) {
        let Some(id) = tree.int(&format!("{CHARS}[{index}].id")).and_then(|id| u32::try_from(id).ok()) else {
            found.push(format!("{CHARS}[{index}].id: unreadable"));
            continue;
        };
        match (slotted(model, id), listed(&tree, id)) {
            (Ok(slots), Some(list)) if slots == list => {}
            (Ok(_), Some(_)) => found.push(format!("character.{id}: spell slots differ from abilityList")),
            (Err(message), _) => found.push(message),
            (_, None) => found.push(format!("character.{id}.abilityList: unreadable")),
        }
    }
    found
}

/// The path of the `abilitySlotDataList` string of the character at `index`.
fn slot_list_path(index: usize) -> [Segment<'static>; 8] {
    [
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("ownedCharacterList"),
        Segment::Doc,
        Segment::Key("target"),
        Segment::Index(index),
        Segment::Doc,
        Segment::Key("abilitySlotDataList"),
    ]
}

/// One element of `abilitySlotDataList.target`: the level and a `slotInfo`
/// dictionary with `keys` and the raw JSON `values`.
pub(crate) fn level_element(level: u32, keys: &[u32], values: &[&str]) -> String {
    let keys: Vec<String> = keys.iter().map(ToString::to_string).collect();
    let dictionary = format!("{{\"keys\":[{}],\"values\":[{}]}}", keys.join(","), values.join(","));
    embed(&format!("{{\"level\":{level},\"slotInfo\":{}}}", embed(&dictionary)))
}

/// The JSON of an ability object, as a slot value.
pub(crate) fn ability(id: u32) -> String {
    embed(&format!("{{\"abilityId\":{id},\"contentId\":{},\"skillLevel\":0}}", id + 208))
}

/// A model over the standard slot whose character `id` (1 to 4) holds
/// `elements` as its `abilitySlotDataList`.
pub(crate) fn model_with_slots(id: u32, elements: &[String]) -> (Ff1Model, String) {
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    let index = usize::try_from(id).unwrap() - 1;
    let list = embed(&format!("{{\"target\":[{}]}}", elements.join(",")));
    document.set_raw(&slot_list_path(index), &list).unwrap();
    let text = document.to_json();
    (Ff1Model::new(document, neoffse_data::tables_for(GAME).unwrap()), text)
}

/// The id of the spell slot `level`, `slot` of character `id`.
pub(crate) fn spell(id: u32, level: u32, slot: u32) -> FieldId {
    FieldId::new(names::CHARACTER).with_id(id).with_name(names::SPELL).with_id(level).with_id(slot)
}
