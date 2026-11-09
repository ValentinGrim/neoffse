//! The spell Level field and the Magic section of the FF II model against the
//! synthetic slot: the level comes from the row of the `abilityList` entry
//! (`docs/formats/pixel-remaster.md#spells-in-ff-ii`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::character_path;
use common::ff2_save::{SlotData, Spell, model, standard_model, tables};
use common::ff2_spell::{small_tables, spell_rows};
use neoffse_core::{CapPolicy, Capability, Edit, EditError, FieldId, Shape, TableKind, Value};
use neoffse_data::{GameTables, Language};
use neoffse_format_pr::document::{Document, Scalar, Segment};
use neoffse_format_pr::ff2::{Ff2Model, names};

fn level(model: &mut Ff2Model, id: u32, chain: u32) -> Result<Value, EditError> {
    model.get(&names::spell_field(id, chain))
}

/// The path to a key of the `abilityList` entry `index` of the character at `position`.
fn entry_key(position: usize, index: usize, key: &'static str) -> Vec<Segment<'static>> {
    character_path(
        position,
        &[
            Segment::Doc,
            Segment::Key("abilityList"),
            Segment::Doc,
            Segment::Key("target"),
            Segment::Index(index),
            Segment::Doc,
            Segment::Key(key),
        ],
    )
}

fn name_of(chain: u32, language: Language) -> String {
    let first = tables().spell_ability(chain, 1).unwrap();
    tables().entry(TableKind::Ability, first).unwrap().name(language).unwrap().to_owned()
}

#[test]
fn the_levels_come_from_the_rows() {
    let mut model = standard_model();
    for character in SlotData::standard().characters {
        for spell in &character.spells {
            assert_eq!(
                level(&mut model, character.id, spell.chain).unwrap(),
                Value::Int(i64::from(spell.row_level)),
                "{} {}",
                character.id,
                spell.chain
            );
        }
    }
    // Both ends of a chain: character 1 holds level 1 of chain 1 and level 16 of chain 2.
    assert_eq!(level(&mut model, 1, 1).unwrap(), Value::Int(1));
    assert_eq!(level(&mut model, 1, 2).unwrap(), Value::Int(16));
}

#[test]
fn a_mismatched_pair_reads_the_row_level() {
    let mut model = standard_model();
    // Character 2 holds chain 5 at row level 1 with the values 456 (list) and 400 (copy), after 711 and two spells.
    assert_eq!(model.read(&entry_key(1, 3, "skillLevel")).unwrap(), Scalar::Int(456));
    assert_eq!(
        model.read(&entry_key(1, 3, "abilityId")).unwrap(),
        Scalar::Int(i64::from(tables().spell_ability(5, 1).unwrap()))
    );
    let before = model.document().to_json();
    assert_eq!(level(&mut model, 2, 5).unwrap(), Value::Int(1));
    model.view(Language::En).unwrap();
    assert_eq!(model.document().to_json(), before, "a read wrote");
}

#[test]
fn a_duplicate_chain_entry_reads_the_first() {
    for (spells, expected) in [
        (vec![Spell::consistent(0, 5), Spell::consistent(0, 9)], 5),
        (vec![Spell::consistent(0, 9), Spell::consistent(0, 5)], 9),
    ] {
        let mut slot = SlotData::standard();
        slot.characters[0].spells = spells;
        let mut model = model(&slot.render());
        assert_eq!(level(&mut model, 1, 0).unwrap(), Value::Int(expected));
        let view = model.view(Language::En).unwrap();
        let fields = &view.section(Capability::Magic).unwrap().entities[0].fields;
        assert_eq!(fields.len(), 1, "the second entry of a chain has no field");
    }
}

fn model_with_first_entry(ability: i64) -> Ff2Model {
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    document.set(&entry_key(0, 0, "abilityId"), &Scalar::Int(ability)).unwrap();
    Ff2Model::new(document, tables())
}

#[test]
fn an_ability_outside_the_chains_has_no_field() {
    let mut standard = standard_model();
    assert_eq!(standard.read(&entry_key(0, 0, "abilityId")).unwrap(), Scalar::Int(711));
    let view = standard.view(Language::En).unwrap();
    let ids: Vec<FieldId> = view.section(Capability::Magic).unwrap().entities[0]
        .fields
        .iter()
        .map(|field| field.id.clone())
        .collect();
    assert_eq!(ids, [names::spell_field(1, 0), names::spell_field(1, 1), names::spell_field(1, 2)]);
    // A chain with no entry, and a chain that does not exist.
    for chain in [3, 39, 40, u32::MAX] {
        assert!(matches!(level(&mut standard, 1, chain), Err(EditError::UnknownField { .. })), "chain {chain}");
    }
    // The ids next to the ends of the 640 rows: 645 and 4 are outside every chain, 644 is level 16 of chain 39.
    // Ability 5 opens chain 0, which character 1 already owns, so the first entry wins and the count stays.
    let cases = [(645, 3, None), (644, 4, Some((39, 16))), (4, 3, None), (5, 3, Some((0, 1)))];
    for (ability, count, expected) in cases {
        let mut model = model_with_first_entry(ability);
        let view = model.view(Language::En).unwrap();
        assert_eq!(view.section(Capability::Magic).unwrap().entities[0].fields.len(), count, "ability {ability}");
        if let Some((chain, row)) = expected {
            assert_eq!(level(&mut model, 1, chain).unwrap(), Value::Int(row), "ability {ability}");
        }
    }
}

#[test]
fn a_character_without_a_spell_list_owns_no_spell() {
    let mut slot = SlotData::standard();
    slot.characters[1].omit = Some("abilityList");
    let mut model = model(&slot.render());
    assert!(matches!(level(&mut model, 2, 3), Err(EditError::UnknownField { .. })));
    let view = model.view(Language::En).unwrap();
    let entities = &view.section(Capability::Magic).unwrap().entities;
    // Four Level entities, then four raw pages: the raw page of character 2 still holds its dictionary and orders.
    assert_eq!(entities.len(), 8);
    assert_eq!(entities[1].fields, []);
    assert_eq!(entities[0].fields.len(), 3);
}

#[test]
fn the_meta_describes_a_level() {
    let model = standard_model();
    let meta = model.field_meta(&names::spell_field(1, 0)).unwrap();
    assert_eq!(meta.label, "Spell level");
    assert_eq!(meta.shape, Shape::Int { min: 1, max: 16 });
    assert!(!meta.read_only);
    assert_eq!(meta.cap, None);
    assert_eq!(meta.tables, []);
    assert!(!model.field_locked(&names::spell_field(1, 0)).unwrap());
}

#[test]
fn the_magic_section_holds_one_entity_per_character() {
    let mut slot = SlotData::standard();
    slot.characters.reverse();
    slot.characters[2].name = String::new();
    let mut model = model(&slot.render());
    let view = model.view(Language::En).unwrap();
    let section = view.section(Capability::Magic).unwrap();
    let ids: Vec<FieldId> = section.entities.iter().map(|entity| entity.id.clone()).collect();
    let expected: Vec<FieldId> = [4, 3, 2, 1].map(|id| FieldId::new(names::CHARACTER).with_id(id)).to_vec();
    // The raw pages follow the four Level entities, in the same order.
    assert_eq!(ids[..4], expected);
    assert_eq!(ids[4..], expected.iter().map(|id| id.clone().with_name(names::RAW_SPELLS)).collect::<Vec<_>>());
    let labels: Vec<&str> = section.entities.iter().take(4).map(|entity| entity.label.as_str()).collect();
    assert_eq!(labels, ["Élan", "Tane", "Character 2", "Ivo"]);
    // Character 2 holds chains 3, 4, 5, 6 in this list order.
    let fields = &section.entities[2].fields;
    let chains: Vec<FieldId> = [3, 4, 5, 6].map(|chain| names::spell_field(2, chain)).to_vec();
    assert_eq!(fields.iter().map(|field| field.id.clone()).collect::<Vec<_>>(), chains);
    assert_eq!(
        fields.iter().map(|field| field.value.clone()).collect::<Vec<_>>(),
        [2, 8, 1, 11].map(Value::Int)
    );
    for field in fields {
        assert_eq!(Some(field.meta.shape), model.field_meta(&field.id).map(|meta| meta.shape));
    }
}

#[test]
fn a_level_is_labelled_with_the_spell_name() {
    let mut model = standard_model();
    let view = model.view(Language::En).unwrap();
    let fields = &view.section(Capability::Magic).unwrap().entities[3].fields;
    assert_eq!(fields.len(), 16);
    for field in fields {
        let chain = u32::try_from(fields.iter().position(|other| other.id == field.id).unwrap()).unwrap();
        assert_eq!(field.meta.label, name_of(chain, Language::En));
    }
    let french = model.view(Language::Fr).unwrap();
    let labelled = &french.section(Capability::Magic).unwrap().entities[0].fields[0];
    assert_eq!(labelled.meta.label, name_of(0, Language::Fr));
    assert_ne!(name_of(0, Language::En), name_of(0, Language::Fr));
}

#[test]
fn a_raw_set_of_a_level_is_read_only_under_both_policies() {
    let mut model = standard_model();
    let before = model.document().to_json();
    // An owned spell, a chain without an entry and a character without a row: the refusal precedes any read.
    for (id, chain) in [(1, 0), (1, 3), (99, 0)] {
        for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
            for value in [5, 0, 17] {
                let edit = Edit::set(names::spell_field(id, chain), Value::Int(value));
                assert!(matches!(model.apply(&edit, policy), Err(EditError::ReadOnly { .. })), "{id} {chain} {value}");
            }
        }
    }
    assert_eq!(model.document().to_json(), before);
}

fn nameless_tables() -> &'static GameTables {
    small_tables(
        &spell_rows(4, 3),
        "first = [1, 3]\nlength = 2\n",
        "[bounds]\nMAX_GIL = 999999\nMAX_MAGICAL_LEVEL = 2\nMAX_WEAPON_LEVEL = 2\n",
    )
}

#[test]
fn a_chain_without_a_name_is_labelled_entry() {
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    // Character 1 holds 711 first; the next two entries become row 2 of chain 0 and row 1 of chain 1.
    for (index, ability) in [(1, 2), (2, 3)] {
        document.set(&entry_key(0, index, "abilityId"), &Scalar::Int(ability)).unwrap();
    }
    let mut model = Ff2Model::new(document, nameless_tables());
    let view = model.view(Language::En).unwrap();
    let fields = &view.section(Capability::Magic).unwrap().entities[0].fields;
    let found: Vec<(FieldId, String, Value)> = fields
        .iter()
        .map(|field| (field.id.clone(), field.meta.label.clone(), field.value.clone()))
        .collect();
    assert_eq!(
        found,
        [
            (names::spell_field(1, 0), "Entry 1".to_owned(), Value::Int(2)),
            (names::spell_field(1, 1), "Cure".to_owned(), Value::Int(1)),
        ]
    );
    assert_eq!(model.field_meta(&names::spell_field(1, 0)).unwrap().shape, Shape::Int { min: 1, max: 2 });
}
