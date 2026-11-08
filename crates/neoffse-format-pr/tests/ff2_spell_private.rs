//! The spell Level edit on the private FF II slot files. The folder is
//! read-only input: each slot is decoded into memory, edited there and
//! encoded to memory, and the tests write nothing. A failure names the label,
//! a character position and a field path, never a value, a length or a name.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::collections::BTreeMap;

use common::character_path;
use common::ff1_format::one_file;
use common::ff1_real::{RealSlot, must, must_some};
use common::ff2_real::slots;
use common::ff2_save::{content_of, tables};
use common::ff2_spell::{Where, level_edit};
use common::json_diff::{Change, assert_changes_with, diff_trees, layout_chain};
use common::json_tree::Tree;
use neoffse_core::{CapPolicy, Capability, EditError, FieldView, Key, SaveDocument, SaveFormat, Value};
use neoffse_format_pr::container::{Decoded, decode};
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Format, names};

/// The three stored values of an ability entry: row, content id, proficiency.
type Entry = [i64; 3];
const PARTS: [&str; 3] = ["abilityId", "contentId", "skillLevel"];
const KEY_OF_MAGIC: i64 = 3;

/// The stored places of one character, read from its raw page.
#[derive(Default)]
struct Places {
    list: Vec<Entry>,
    copy: Vec<Entry>,
    addition: Vec<i64>,
    sort: Vec<i64>,
}

fn index_of(part: &str) -> Option<usize> {
    [names::ABILITY_ID, names::CONTENT_ID, names::PROFICIENCY].iter().position(|name| *name == part)
}

fn int(field: &FieldView) -> i64 {
    match field.value {
        Value::Int(value) => value,
        _ => panic!("{}: a place is no integer", field.id),
    }
}

/// Groups the fields of a raw page by place. An entry that lacks a part is a
/// failure: the real slots store all three.
fn places_of(fields: &[FieldView], label: &str) -> Places {
    let mut entries: [BTreeMap<u32, [Option<i64>; 3]>; 2] = [BTreeMap::new(), BTreeMap::new()];
    let mut orders: [BTreeMap<u32, i64>; 2] = [BTreeMap::new(), BTreeMap::new()];
    for field in fields {
        match field.id.keys() {
            [_, _, Key::Name(group), Key::Id(position), Key::Name(part)] => {
                let which = usize::from(*group == names::DICTIONARY);
                let slot = entries[which].entry(*position).or_default();
                slot[must_some(index_of(part), &format!("{label}: {}: unknown part", field.id))] = Some(int(field));
            }
            [_, _, Key::Name(order), Key::Id(position)] => {
                orders[usize::from(*order == names::SORT_ORDER)].insert(*position, int(field));
            }
            _ => panic!("{label}: {}: unexpected place path", field.id),
        }
    }
    let complete = |map: &BTreeMap<u32, [Option<i64>; 3]>| -> Vec<Entry> {
        map.values()
            .map(|parts| parts.map(|part| must_some(part, &format!("{label}: an ability entry lacks a part"))))
            .collect()
    };
    Places {
        list: complete(&entries[0]),
        copy: complete(&entries[1]),
        addition: orders[0].values().copied().collect(),
        sort: orders[1].values().copied().collect(),
    }
}

fn chain_of(ability: i64) -> Option<u32> {
    (5..=644).contains(&ability).then(|| u32::try_from((ability - 5) / 16).unwrap())
}

/// One owned spell: its first entry in the list and in the copy.
struct Spell {
    chain: u32,
    level: u32,
    list: usize,
    copy: usize,
    mismatched: bool,
}

fn spells(places: &Places) -> Vec<Spell> {
    let mut found: Vec<Spell> = Vec::new();
    for (list, entry) in places.list.iter().enumerate() {
        let Some(chain) = chain_of(entry[0]) else { continue };
        if found.iter().any(|spell| spell.chain == chain) {
            continue;
        }
        let Some(copy) = places.copy.iter().position(|other| chain_of(other[0]) == Some(chain)) else {
            continue;
        };
        let level = u32::try_from((entry[0] - 5) % 16 + 1).unwrap();
        let total = i64::from(tables().skill_total(level).unwrap());
        let mismatched = entry[2] != places.copy[copy][2] || entry[2] != total;
        found.push(Spell {
            chain,
            level,
            list,
            copy,
            mismatched,
        });
    }
    found
}

/// The first spells to edit: every mismatched pair, the first and the last owned spell.
fn chosen(found: &[Spell]) -> Vec<&Spell> {
    let ends = [found.first(), found.last()].into_iter().flatten();
    let mut picked: Vec<&Spell> = Vec::new();
    for spell in found.iter().filter(|spell| spell.mismatched).chain(ends) {
        if picked.iter().all(|other| other.chain != spell.chain) {
            picked.push(spell);
        }
    }
    picked
}

/// The target levels of a spell: its own (a repair or no change), the far end, the next one.
fn targets(level: u32) -> Vec<u32> {
    let mut levels = vec![level, if level > 8 { 1 } else { 16 }, if level < 16 { level + 1 } else { level - 1 }];
    levels.dedup();
    levels
}

/// A path with the list indices blinded, so a message shows no length.
fn blind(path: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for mark in path.chars() {
        match mark {
            '[' => {
                inside = true;
                out.push_str("[#");
            }
            ']' => {
                inside = false;
                out.push(']');
            }
            _ if inside => {}
            _ => out.push(mark),
        }
    }
    out
}

fn first_position(list: &[i64], wanted: i64) -> Option<usize> {
    list.iter().position(|value| *value == wanted)
}

/// The paths the Level edit of `spell` to `target` changes, from the stored
/// places before it and the tables.
fn expected(places: &Places, spell: &Spell, target: u32, character: usize, values: usize) -> Vec<Change> {
    let ability = tables().spell_ability(spell.chain, target).unwrap();
    let new: Entry = [
        i64::from(ability),
        i64::from(content_of(ability)),
        i64::from(tables().skill_total(target).unwrap()),
    ];
    let moved = |old: &Entry| -> Vec<&'static str> {
        let mut parts = Vec::new();
        for (index, part) in PARTS.iter().enumerate() {
            if old[index] != new[index] {
                parts.push(*part);
            }
        }
        parts
    };
    let old_content = places.list[spell.list][1];
    let moves_content = old_content != new[1];
    let at = Where {
        character,
        list: spell.list,
        copy: spell.copy,
        addition: first_position(&places.addition, old_content).filter(|_| moves_content),
        sort: first_position(&places.sort, old_content).filter(|_| moves_content),
    };
    at.changes_of(&moved(&places.list[spell.list]), &moved(&places.copy[spell.copy]), values)
}

fn encoded(document: &dyn SaveDocument, name: &str, label: &str) -> (neoffse_core::SaveFiles, Decoded) {
    let files = must(document.encode(), &format!("{label}: encode failed"));
    let bytes = must_some(files.get(name), &format!("{label}: the encoding lacks the slot file"));
    let decoded = must(decode(bytes), &format!("{label}: decode failed"));
    (files, decoded)
}

/// The character positions with a raw page, with the id and the page's places.
fn pages(document: &mut dyn SaveDocument, label: &str) -> Vec<(usize, u32, Places)> {
    let view = must(document.view(), &format!("{label}: the view does not build"));
    let section = must_some(view.section(Capability::Magic), &format!("{label}: no Magic section"));
    let levels: Vec<u32> = section
        .entities
        .iter()
        .filter_map(|entity| match entity.id.keys() {
            [_, Key::Id(id)] => Some(*id),
            _ => None,
        })
        .collect();
    section
        .entities
        .iter()
        .filter_map(|entity| {
            let [_, Key::Id(id), Key::Name(_)] = entity.id.keys() else { return None };
            let position = must_some(levels.iter().position(|level| level == id), &format!("{label}: a raw page lacks its character"));
            Some((position, *id, places_of(&entity.fields, label)))
        })
        .collect()
}

fn level_of(document: &mut dyn SaveDocument, id: u32, chain: u32, label: &str) -> Option<Value> {
    let view = must(document.view(), &format!("{label}: the view does not build"));
    view.find(&names::spell_field(id, chain)).map(|(_, field)| field.value.clone())
}

fn open(slot: &RealSlot) -> Box<dyn SaveDocument> {
    let format: &dyn SaveFormat = &Ff2Format::new();
    must(format.open(&one_file(&slot.name, slot.bytes.clone())), &format!("{}: open failed", slot.label))
}

#[test]
fn real_level_edits_write_the_places_and_undo_exactly() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let label = slot.label.clone();
        let mut document = open(&slot);
        let (_, original) = encoded(&*document, &slot.name, &label);
        let before = Tree::parse(&original.json);
        for (position, id, places) in pages(&mut *document, &label) {
            let base = format!("userData.ownedCharacterList.target[{position}]");
            let found = spells(&places);
            // Key 3 is present exactly when the character holds a spell, and its list counts the spells.
            let values = before.find_int(&format!("{base}.abilityDictionary.keys"), KEY_OF_MAGIC);
            let spell_entries = places.list.iter().filter(|entry| chain_of(entry[0]).is_some()).count();
            assert!(
                values.is_some() || spell_entries == 0,
                "{label}: character {position}: key 3 is absent while the list holds spells"
            );
            assert!(
                places.copy.len() == spell_entries,
                "{label}: character {position}: the copy count differs from the spell count"
            );
            let Some(values) = values else { continue };
            for spell in chosen(&found) {
                for target in targets(spell.level) {
                    let case = format!("{label}: character {position} chain {} to level {target}", spell.chain);
                    let change = must(
                        document.apply_game(&level_edit(id, spell.chain, i64::from(target))),
                        &format!("{case}: the edit is refused"),
                    );
                    let (files, edited) = encoded(&*document, &slot.name, &case);
                    assert!(edited.container == original.container, "{case}: the container changed");
                    assert!(document.verify(&files).is_ok(), "{case}: verify refuses the encoding");
                    assert!(
                        level_of(&mut *document, id, spell.chain, &case) == Some(Value::Int(i64::from(target))),
                        "{case}: the Level field does not read the new level"
                    );
                    // Exactly the places of the plan changed, and every other byte is as read.
                    let after = Tree::parse(&edited.json);
                    let mut want = expected(&places, spell, target, position, values);
                    want.extend(layout_chain(&before, &want));
                    assert_changes_with(&case, &diff_trees(&before, &after), &want, &|change| blind(&change.to_string()));
                    // Decode, encode, decode gives the same document.
                    assert!(
                        matches!(Document::parse(&edited.json), Ok(parsed) if parsed.to_json() == edited.json),
                        "{case}: the document layer rewrites the text on load"
                    );
                    let reopened = must(Ff2Format::new().open_document(&files), &format!("{case}: reopen failed"));
                    let (_, again) = encoded(&reopened, &slot.name, &case);
                    assert!(
                        again.json == edited.json && again.container == edited.container,
                        "{case}: encode, decode, encode differs"
                    );
                    // Undo restores the text; a spaced file may differ in the layout of the levels the edit rewrote.
                    must(document.apply_all(&change.inverse, CapPolicy::StorageOnly), &format!("{case}: undo refused"));
                    let (_, undone) = encoded(&*document, &slot.name, &case);
                    let layout_only = diff_trees(&before, &Tree::parse(&undone.json)) == layout_chain(&before, &want);
                    assert!(undone.json == original.json || layout_only, "{case}: undo does not restore the text");
                    must(document.apply_all(&change.forward, CapPolicy::StorageOnly), &format!("{case}: redo refused"));
                    let (_, redone) = encoded(&*document, &slot.name, &case);
                    assert!(redone.json == edited.json, "{case}: redo differs");
                    must(
                        document.apply_all(&change.inverse, CapPolicy::StorageOnly),
                        &format!("{case}: second undo refused"),
                    );
                    let (_, last) = encoded(&*document, &slot.name, &case);
                    assert!(last.json == undone.json, "{case}: a second undo differs from the first");
                }
            }
        }
    }
}

/// A copy of the slot whose key-3 entry of the character at `position` is
/// renamed to a key the game never writes: the copy is absent.
#[test]
fn a_real_character_without_its_dictionary_copy_writes_nothing() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut document = open(&slot);
        let label = slot.label.clone();
        for (position, id, places) in pages(&mut *document, &label) {
            let Some(spell) = spells(&places).into_iter().next() else { continue };
            let mut renamed = must(Document::parse(&slot.json), &format!("{label}: the JSON does not parse"));
            let keys = |index: usize| {
                character_path(
                    position,
                    &[
                        Segment::Doc,
                        Segment::Key("abilityDictionary"),
                        Segment::Doc,
                        Segment::Key("keys"),
                        Segment::Index(index),
                    ],
                )
            };
            let found = (0..64).find(|index| matches!(renamed.get(&keys(*index)), Ok(Scalar::Int(KEY_OF_MAGIC))));
            let index = must_some(found, &format!("{label}: character {position}: key 3 not found"));
            must(
                renamed.set(&keys(index), &Scalar::Int(99)),
                &format!("{label}: character {position}: the key does not rename"),
            );
            let mut model = neoffse_format_pr::ff2::Ff2Model::new(renamed, tables());
            let before = model.document().to_json();
            let result = model.apply_game(&level_edit(id, spell.chain, i64::from(spell.level % 16 + 1)));
            assert!(
                matches!(result, Err(EditError::UnknownField { .. })),
                "{label}: character {position}: the missing copy is not UnknownField"
            );
            assert!(model.document().to_json() == before, "{label}: character {position}: a refused edit wrote");
        }
    }
}
