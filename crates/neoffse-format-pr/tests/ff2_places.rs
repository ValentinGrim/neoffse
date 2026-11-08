//! The raw places of a spell on the synthetic FF II slot: positions, the
//! exact inverse of every raw `Set`, the bounds, the locks and the raw page
//! of the Magic section (`docs/formats/pixel-remaster.md#row-and-value-in-ff-ii`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff2_save::{SlotData, content_of, model, standard_model, tables};
use neoffse_core::{CapPolicy, Capability, Edit, EditError, Entity, FieldId, Shape, TableKind, Value};
use neoffse_data::Language;
use neoffse_format_pr::ff2::{Ff2Model, names};

const ENFORCE: CapPolicy = CapPolicy::Enforce;
const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;

fn ability(id: u32, position: u32, part: &'static str) -> FieldId {
    names::place(id, names::ABILITY, position, part)
}

fn dictionary(id: u32, position: u32, part: &'static str) -> FieldId {
    names::place(id, names::DICTIONARY, position, part)
}

fn set(field: &FieldId, value: i64) -> Edit {
    Edit::set(field.clone(), Value::Int(value))
}

fn int(model: &mut Ff2Model, field: &FieldId) -> i64 {
    match model.get(field).unwrap() {
        Value::Int(value) => value,
        other => panic!("{field}: {other:?}"),
    }
}

/// The raw page of character `id` in the Magic section.
fn raw_page(model: &mut Ff2Model, id: u32) -> Entity {
    let view = model.view(Language::En).unwrap();
    let wanted = FieldId::new(names::CHARACTER).with_id(id).with_name(names::RAW_SPELLS);
    view.section(Capability::Magic)
        .unwrap()
        .entities
        .iter()
        .find(|entity| entity.id == wanted)
        .cloned()
        .unwrap()
}

fn name_of(ability: u32) -> String {
    tables().entry(TableKind::Ability, ability).unwrap().name(Language::En).unwrap().to_owned()
}

#[test]
fn the_places_sit_at_their_positions() {
    let mut model = standard_model();
    // Character 1 holds 711, then chain 0 at level 5, chain 1 at level 1, chain 2 at level 16.
    assert_eq!(int(&mut model, &ability(1, 0, names::ABILITY_ID)), 711);
    let first = tables().spell_ability(0, 5).unwrap();
    assert_eq!(int(&mut model, &ability(1, 1, names::ABILITY_ID)), i64::from(first));
    assert_eq!(int(&mut model, &ability(1, 1, names::PROFICIENCY)), i64::from(tables().skill_total(5).unwrap()));
    // The dictionary has no entry for 711, so its position 0 is the first spell.
    assert_eq!(int(&mut model, &dictionary(1, 0, names::ABILITY_ID)), i64::from(first));
    let content = content_of(first);
    assert_eq!(int(&mut model, &dictionary(1, 0, names::CONTENT_ID)), i64::from(content));
    assert_eq!(int(&mut model, &names::order_place(1, names::ADDITION_ORDER, 1)), i64::from(content));
    // The sort order lists the spells in reverse: position 0 is the last spell.
    let last = content_of(tables().spell_ability(2, 16).unwrap());
    assert_eq!(int(&mut model, &names::order_place(1, names::SORT_ORDER, 0)), i64::from(last));
}

#[test]
fn ability_711_has_its_entry_and_the_ability_table_names_the_row() {
    let mut model = standard_model();
    let page = raw_page(&mut model, 1);
    let field = &page.fields[0];
    assert_eq!(field.id, ability(1, 0, names::ABILITY_ID));
    assert_eq!(field.value, Value::Int(711));
    assert_eq!(field.meta.label, "List 1, Entry 711, row");
    assert_eq!(field.meta.tables, [TableKind::Ability]);
    assert!(tables().entry(TableKind::Ability, 711).is_some());
    // The same row is a plain field under the model's own meta.
    assert_eq!(model.field_meta(&field.id).unwrap().tables, [TableKind::Ability]);
}

#[test]
fn the_labels_follow_the_document_order() {
    let mut model = standard_model();
    let page = raw_page(&mut model, 1);
    assert_eq!(page.label, "Ivo, raw spells");
    let fire = name_of(tables().spell_ability(0, 5).unwrap());
    let labels: Vec<&str> = page.fields.iter().map(|field| field.meta.label.as_str()).collect();
    assert_eq!(
        labels[..3],
        ["List 1, Entry 711, row", "List 1, Entry 711, content", "List 1, Entry 711, value"]
    );
    assert_eq!(
        labels[3..6],
        [
            format!("List 2, {fire}, row"),
            format!("List 2, {fire}, content"),
            format!("List 2, {fire}, value")
        ]
    );
    // 4 list entries of 3 fields, 3 copies of 3, 4 addition places, 3 sort places.
    assert_eq!(labels[12], format!("Copy 1, {fire}, row"));
    assert_eq!(labels[21], "Addition order 1");
    assert_eq!(labels[23], "Addition order 3");
    assert_eq!(labels[25], "Sort order 1");
    assert_eq!(labels[27], "Sort order 3");
}

#[test]
fn a_place_count_follows_the_entry_counts() {
    let mut model = standard_model();
    // 3 per list entry (spells and 711), 3 per copy, one per order place: 8 * spells + 4.
    for (id, spells) in [(1, 3), (2, 4), (3, 1), (4, 16)] {
        assert_eq!(raw_page(&mut model, id).fields.len(), 8 * spells + 4, "character {id}");
    }
    assert_eq!(raw_page(&mut model, 4).fields.len(), 132);
}

#[test]
fn a_position_past_the_list_is_unknown() {
    let mut model = standard_model();
    let past = [
        ability(1, 99, names::PROFICIENCY),
        // Character 1 holds four list entries and three copies: no copy sits at the 711 position.
        ability(1, 4, names::ABILITY_ID),
        dictionary(1, 3, names::ABILITY_ID),
        names::order_place(1, names::ADDITION_ORDER, 99),
        names::order_place(1, names::SORT_ORDER, 3),
    ];
    for field in past {
        assert!(matches!(model.get(&field), Err(EditError::UnknownField { .. })), "{field}");
        let edit = set(&field, 1);
        assert!(matches!(model.apply(&edit, STORAGE_ONLY), Err(EditError::UnknownField { .. })), "{field}");
    }
    // A part or a group the vocabulary lacks.
    let lacking = FieldId::new(names::CHARACTER)
        .with_id(1)
        .with_name(names::ABILITY)
        .with_id(0)
        .with_name("level");
    assert!(model.field_meta(&lacking).is_none());
}

#[test]
fn a_one_sided_proficiency_edit_undoes_exactly() {
    let mut model = standard_model();
    let before = model.document().to_json();
    let field = ability(1, 1, names::PROFICIENCY);
    let copy = dictionary(1, 0, names::PROFICIENCY);
    let copy_before = int(&mut model, &copy);
    let inverse = model.apply(&set(&field, 456), STORAGE_ONLY).unwrap();
    assert_ne!(model.document().to_json(), before);
    assert_eq!(int(&mut model, &field), 456);
    assert_eq!(int(&mut model, &copy), copy_before, "the copy stays");
    // The row is left alone: the Level field still reads the row.
    assert_eq!(model.get(&names::spell_field(1, 0)).unwrap(), Value::Int(5));
    model.apply(&inverse, STORAGE_ONLY).unwrap();
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn an_ability_id_edit_keeps_its_field_and_undoes_exactly() {
    let mut model = standard_model();
    let before = model.document().to_json();
    let field = ability(1, 1, names::ABILITY_ID);
    let old = int(&mut model, &field);
    // An id of another chain: chain 9, level 3.
    let other = i64::from(tables().spell_ability(9, 3).unwrap());
    let inverse = model.apply(&set(&field, other), STORAGE_ONLY).unwrap();
    assert_eq!(inverse, set(&field, old), "the inverse names the same position");
    assert_eq!(int(&mut model, &field), other);
    // The next edit at that position still reaches the entry.
    let proficiency = ability(1, 1, names::PROFICIENCY);
    let second = model.apply(&set(&proficiency, 7), STORAGE_ONLY).unwrap();
    model.apply(&second, STORAGE_ONLY).unwrap();
    // The chain the entry left has no Level field any more, the one it joined has.
    assert!(matches!(model.get(&names::spell_field(1, 0)), Err(EditError::UnknownField { .. })));
    assert_eq!(model.get(&names::spell_field(1, 9)).unwrap(), Value::Int(3));
    model.apply(&inverse, STORAGE_ONLY).unwrap();
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn an_order_place_edit_undoes_exactly() {
    let mut model = standard_model();
    let before = model.document().to_json();
    for order in [names::ADDITION_ORDER, names::SORT_ORDER] {
        let field = names::order_place(1, order, 1);
        let inverse = model.apply(&set(&field, 4242), STORAGE_ONLY).unwrap();
        assert_eq!(int(&mut model, &field), 4242, "{order}");
        model.apply(&inverse, STORAGE_ONLY).unwrap();
        assert_eq!(model.document().to_json(), before, "{order}");
    }
}

#[test]
fn enforce_holds_the_proficiency_cap_and_the_ability_table() {
    let mut model = standard_model();
    let proficiency = ability(1, 1, names::PROFICIENCY);
    let meta = model.field_meta(&proficiency).unwrap();
    assert_eq!(meta.cap, Some(860));
    assert_eq!(
        meta.shape,
        Shape::Int {
            min: 0,
            max: i64::from(i32::MAX)
        }
    );
    model.apply(&set(&proficiency, 860), ENFORCE).unwrap();
    assert!(matches!(
        model.apply(&set(&proficiency, 861), ENFORCE),
        Err(EditError::AboveCap { cap: 860, .. })
    ));
    assert!(matches!(model.apply(&set(&proficiency, -1), ENFORCE), Err(EditError::OutOfRange { .. })));
    assert!(model.apply(&set(&proficiency, 861), STORAGE_ONLY).is_ok());
    let id = ability(1, 1, names::ABILITY_ID);
    let before = model.document().to_json();
    assert!(matches!(model.apply(&set(&id, 100_000), ENFORCE), Err(EditError::UnknownId { .. })));
    assert_eq!(model.document().to_json(), before, "a refusal writes nothing");
    assert!(model.apply(&set(&id, 100_000), STORAGE_ONLY).is_ok());
    // Ability 711 is listed, nameless, and Enforce accepts it.
    assert!(model.apply(&set(&id, 711), ENFORCE).is_ok());
}

#[test]
fn every_place_is_locked_and_the_game_interface_refuses_it() {
    let mut model = standard_model();
    let before = model.document().to_json();
    for id in 1..=4 {
        for field in raw_page(&mut model, id).fields {
            assert!(model.field_locked(&field.id).unwrap(), "{}", field.id);
            assert!(matches!(model.apply_game(&set(&field.id, 5)), Err(EditError::ReadOnly { .. })), "{}", field.id);
        }
    }
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn the_view_reads_back_the_model() {
    let mut model = standard_model();
    for id in 1..=4 {
        for field in raw_page(&mut model, id).fields {
            assert_eq!(model.get(&field.id).unwrap(), field.value, "{}", field.id);
            assert_eq!(Some(field.meta.shape), model.field_meta(&field.id).map(|meta| meta.shape), "{}", field.id);
        }
    }
}

#[test]
fn a_character_without_the_dictionary_has_no_copy_places() {
    let mut slot = SlotData::standard();
    slot.characters[0].omit = Some("abilityDictionary");
    let mut model = model(&slot.render());
    // 4 list entries of 3, 4 addition places, 3 sort places.
    assert_eq!(raw_page(&mut model, 1).fields.len(), 12 + 4 + 3);
    let copy = dictionary(1, 0, names::ABILITY_ID);
    // A key the character lacks reads as a `Format` error with a missing key, which the view treats as absent.
    assert!(matches!(model.get(&copy), Err(EditError::Format { .. })));
}

#[test]
fn an_entity_with_no_place_is_left_out() {
    let mut slot = SlotData::standard();
    let character = &mut slot.characters[0];
    character.omit = Some("abilityList");
    character.spells = Vec::new();
    character.addition_order = Some(Vec::new());
    character.sort_order = Some(Vec::new());
    let mut model = model(&slot.render());
    let view = model.view(Language::En).unwrap();
    let gone = FieldId::new(names::CHARACTER).with_id(1).with_name(names::RAW_SPELLS);
    let entities = &view.section(Capability::Magic).unwrap().entities;
    assert!(entities.iter().all(|entity| entity.id != gone));
    assert_eq!(entities.len(), 4 + 3, "four Level entities and the raw pages of the other three");
}
