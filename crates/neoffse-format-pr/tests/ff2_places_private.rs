//! The raw spell places on the private FF II slot files. The folder is
//! read-only input and the tests write nothing. A failure names the label, a
//! character position and a field path, never a value, a length or a name:
//! the files hold personal data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_path;
use common::ff2_real::slots;
use common::private::assert_fields_read_back;
use common::real_slots::{must, must_some};
use neoffse_core::{Capability, FieldId};
use neoffse_data::Language;
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Model, names};

/// The length of the array at `tail` under the character at `position`.
fn length(document: &mut Document, position: usize, tail: &[Segment<'static>], what: &str) -> usize {
    let mut path = vec![Segment::Doc];
    path.extend_from_slice(tail);
    must(document.array_len(&character_path(position, &path)), what)
}

/// The entry count of the Magic command's list in `abilityDictionary`; 0 for
/// a character whose dictionary holds no such key.
fn dictionary_length(document: &mut Document, position: usize, what: &str) -> usize {
    let keys = character_path(position, &[Segment::Doc, Segment::Key("abilityDictionary"), Segment::Doc, Segment::Key("keys")]);
    let count = must(document.array_len(&keys), what);
    let found = (0..count).find(|index| {
        let mut key = keys.clone();
        key.push(Segment::Index(*index));
        matches!(document.get(&key), Ok(Scalar::Int(3)))
    });
    let Some(index) = found else { return 0 };
    let tail = [
        Segment::Key("abilityDictionary"),
        Segment::Doc,
        Segment::Key("values"),
        Segment::Index(index),
        Segment::Doc,
        Segment::Key("target"),
    ];
    length(document, position, &tail, what)
}

#[test]
fn real_place_counts_follow_the_entry_counts() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut document = must(Document::parse(&slot.json), &format!("{}: the JSON does not parse", slot.label));
        let mut model = slot.model::<Ff2Model>();
        let view = must(model.view(Language::En), &format!("{}: the view does not build", slot.label));
        let section = must_some(view.section(Capability::Magic), &format!("{}: no Magic section", slot.label));
        let ids = must(
            model.rows(&FieldId::new(names::CHARACTER)),
            &format!("{}: character rows do not read", slot.label),
        );
        for (position, id) in ids.into_iter().enumerate() {
            let what = |key: &str| format!("{}: character {position}: {key} does not read", slot.label);
            let list = |key: &'static str| [Segment::Key(key), Segment::Doc, Segment::Key("target")];
            let abilities = length(&mut document, position, &list("abilityList"), &what("abilityList"));
            let copies = dictionary_length(&mut document, position, &what("abilityDictionary"));
            let addition = length(
                &mut document,
                position,
                &list("additionOrderOwnedAbilityIds"),
                &what("additionOrderOwnedAbilityIds"),
            );
            let sort = length(&mut document, position, &list("sortOrderOwnedAbilityIds"), &what("sortOrderOwnedAbilityIds"));
            let expected = 3 * abilities + 3 * copies + addition + sort;
            let wanted = FieldId::new(names::CHARACTER).with_id(id).with_name(names::RAW_SPELLS);
            let found = section.entities.iter().find(|entity| entity.id == wanted);
            let count = found.map_or(0, |entity| entity.fields.len());
            assert!(
                count == expected,
                "{}: character {position}: the raw page does not match the entry counts",
                slot.label
            );
        }
    }
}

#[test]
fn real_raw_pages_read_back_through_the_model() {
    let Some(slots) = slots() else { return };
    for slot in slots {
        let mut model = slot.model::<Ff2Model>();
        let view = must(model.view(Language::En), &format!("{}: the view does not build", slot.label));
        let section = must_some(view.section(Capability::Magic), &format!("{}: no Magic section", slot.label));
        let pages = section.entities.iter().filter(|entity| entity.id.keys().len() == 3);
        assert_fields_read_back(&slot.label, &mut model, pages);
        assert!(model.document().to_json() == slot.json, "{}: the view changed the text", slot.label);
    }
}
