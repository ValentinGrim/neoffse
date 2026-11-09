//! The spell reads on the private FF II slot files. The folder is read-only
//! input and the tests write nothing. A failure names the label, a character
//! position and a field path, never a value, a length or a name: the files
//! hold personal data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_path;
use common::ff2_real::slots;
use common::real_slots::{RealSlot, must, must_some};
use neoffse_core::{Capability, FieldId, Value};
use neoffse_data::Language;
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Model, names};

/// The ability 711 entry that every FF II character holds
/// (`docs/formats/pixel-remaster.md#spells-in-ff-ii`).
const INTERNAL: i64 = 711;

/// The `abilityId`s of the `abilityList` of the character at `position`, in
/// list order.
fn listed(document: &mut Document, position: usize, label: &str) -> Vec<i64> {
    let list = character_path(position, &[Segment::Doc, Segment::Key("abilityList"), Segment::Doc, Segment::Key("target")]);
    let len = must(document.array_len(&list), &format!("{label}: character {position}: abilityList does not read"));
    (0..len)
        .map(|index| {
            let mut path = list.clone();
            path.extend([Segment::Index(index), Segment::Doc, Segment::Key("abilityId")]);
            match must(
                document.get(&path),
                &format!("{label}: character {position}: abilityList entry {index} does not read"),
            ) {
                Scalar::Int(id) => id,
                _ => panic!("{label}: character {position}: abilityList entry {index} has no integer abilityId"),
            }
        })
        .collect()
}

/// The level of a chain row by the law of the document: `(id - 5) mod 16 + 1`.
fn law_level(ability: i64) -> i64 {
    (ability - 5).rem_euclid(16) + 1
}

fn in_chains(ability: i64) -> bool {
    (5..=644).contains(&ability)
}

/// The decoded document of `slot`, a model over it and the character ids in
/// document order.
fn open(slot: &RealSlot) -> (Document, Ff2Model, Vec<u32>) {
    let document = must(Document::parse(&slot.json), &format!("{}: the JSON does not parse", slot.label));
    let mut model = slot.model::<Ff2Model>();
    let ids = must(
        model.rows(&FieldId::new(names::CHARACTER)),
        &format!("{}: character rows do not read", slot.label),
    );
    (document, model, ids)
}

/// The chain rows of the character at `position`, the first of each chain, in
/// list order.
fn first_of_each_chain(document: &mut Document, position: usize, label: &str) -> Vec<i64> {
    let mut chains = Vec::new();
    let mut rows = Vec::new();
    for ability in listed(document, position, label).into_iter().filter(|ability| in_chains(*ability)) {
        let chain = (ability - 5).div_euclid(16);
        if !chains.contains(&chain) {
            chains.push(chain);
            rows.push(ability);
        }
    }
    rows
}

#[test]
fn real_ability_lists_hold_chain_rows_and_one_internal_entry() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let (mut document, _, ids) = open(&slot);
        for position in 0..ids.len() {
            let abilities = listed(&mut document, position, &slot.label);
            for ability in &abilities {
                assert!(
                    *ability == INTERNAL || in_chains(*ability),
                    "{}: character {position}: an abilityList entry is in no chain and is not the internal entry",
                    slot.label
                );
            }
            let internal = abilities.iter().filter(|ability| **ability == INTERNAL).count();
            assert!(internal == 1, "{}: character {position}: the internal entry does not occur once", slot.label);
        }
    }
}

#[test]
fn real_levels_equal_the_row_law() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let (mut document, mut model, ids) = open(&slot);
        for (position, id) in ids.into_iter().enumerate() {
            for ability in first_of_each_chain(&mut document, position, &slot.label) {
                let chain = u32::try_from((ability - 5).div_euclid(16)).unwrap();
                let field = names::spell_field(id, chain);
                let found = must(model.get(&field), &format!("{}: {field} does not read", slot.label));
                assert!(found == Value::Int(law_level(ability)), "{}: {field} differs from the row law", slot.label);
            }
        }
    }
}

#[test]
fn real_magic_sections_list_the_owned_spells() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let (mut document, mut model, ids) = open(&slot);
        let view = must(model.view(Language::En), &format!("{}: the view does not build", slot.label));
        let section = must_some(view.section(Capability::Magic), &format!("{}: no Magic section", slot.label));
        // The Level entities come first, one per character; the raw pages follow.
        for (position, entity) in section.entities.iter().take(ids.len()).enumerate() {
            let owned = first_of_each_chain(&mut document, position, &slot.label).len();
            assert!(
                entity.fields.len() == owned,
                "{}: character {position}: the Magic fields do not match the owned chains",
                slot.label
            );
        }
    }
}
