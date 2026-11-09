//! The FF II keywords: `userData.ownedKeyWaordList.target`, a flat list of
//! content ids (`docs/formats/pixel-remaster.md#keywords-in-ff-ii`). The game
//! interface adds one at the end, as `AddKeyWord` does, and refuses a removal;
//! the raw interface removes one and its inverse puts it back at its index.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff2_save::{SlotData, ids, keywords, model, standard_model, tables};
use common::ff2_spell::replay;
use common::history::History;
use common::json_diff::{Kind, added, diff, removed};
use neoffse_core::{CapPolicy, Capability, Edit, EditError, FieldId, RowId, TableKind, Value};
use neoffse_data::Language;
use neoffse_format_pr::document::{Document, Segment};
use neoffse_format_pr::ff2::{Ff2Format, Ff2Model, names};

const LIST: &str = "userData.ownedKeyWaordList.target";

fn collection() -> FieldId {
    FieldId::new(names::KEYWORDS)
}

fn row(key: u32) -> RowId {
    RowId::new(collection(), key)
}

fn add(key: u32) -> Edit {
    Edit::insert(row(key), None, Vec::new())
}

fn keys(model: &mut Ff2Model) -> Vec<u32> {
    model.rows(&collection()).unwrap()
}

/// A keyword of the table that the standard slot lacks.
fn absent() -> u32 {
    ids(TableKind::Keyword).into_iter().find(|id| !keywords().contains(id)).unwrap()
}

fn target() -> Vec<Segment<'static>> {
    vec![
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("ownedKeyWaordList"),
        Segment::Doc,
        Segment::Key("target"),
    ]
}

/// The standard slot whose keyword array carries its own spacing.
fn spaced_model() -> Ff2Model {
    let mut document = Document::parse(&SlotData::standard().render()).unwrap();
    document.set_raw(&target(), "[ 917 , 918 ,  921 , 919 ]").unwrap();
    Ff2Model::new(document, tables())
}

#[test]
fn the_keyword_table_lists_the_fifteen_rows() {
    assert_eq!(ids(TableKind::Keyword), (917..=931).collect::<Vec<u32>>());
}

#[test]
fn the_rows_read_in_document_order() {
    assert_eq!(keys(&mut standard_model()), keywords());
    let mut slot = SlotData::standard();
    slot.keywords = Vec::new();
    assert_eq!(keys(&mut model(&slot.render())), Vec::<u32>::new());
}

#[test]
fn the_collection_meta_describes_the_keywords() {
    let meta = standard_model().collection_meta(&collection()).unwrap();
    assert_eq!(meta.id, collection());
    assert_eq!(meta.label, "Keywords");
    assert!(meta.insertable && meta.removable);
    assert_eq!(meta.fields.len(), 0);
    assert_eq!(meta.key.tables, [TableKind::Keyword]);
    assert_eq!(meta.key.label, "Keyword");
}

#[test]
fn the_inventory_lists_the_keywords_after_the_key_items() {
    let mut model = standard_model();
    let view = model.view(Language::En).unwrap();
    let section = view.section(Capability::Inventory).unwrap();
    let last = section.entities.len() - keywords().len();
    for (index, key) in keywords().into_iter().enumerate() {
        let entity = &section.entities[last + index];
        assert_eq!(entity.id, *row(key).id());
        assert_eq!(entity.fields.len(), 0);
        let name = tables().entry(TableKind::Keyword, key).and_then(|entry| entry.name(Language::En)).unwrap();
        assert_eq!(entity.label, name);
    }
    let meta = section.collections.iter().find(|meta| meta.id == collection()).unwrap();
    assert_eq!(Some(meta.clone()), model.collection_meta(&collection()));
    // The key items come before the keywords.
    let key_items = FieldId::new(names::KEY_ITEMS);
    let first_key_item = section.entities.iter().position(|entity| entity.id.starts_with(&key_items)).unwrap();
    assert!(first_key_item < last);
}

#[test]
fn a_keyword_the_table_lacks_reads_as_keyword_and_its_id() {
    let mut model = standard_model();
    model.apply(&add(5000), CapPolicy::StorageOnly).unwrap();
    let view = model.view(Language::En).unwrap();
    let section = view.section(Capability::Inventory).unwrap();
    let entity = section.entities.iter().find(|entity| entity.id == *row(5000).id()).unwrap();
    assert_eq!(entity.label, "Keyword 5000");
}

#[test]
fn the_game_interface_appends_a_keyword_the_save_lacks() {
    let mut model = standard_model();
    let before = model.document().to_json();
    let key = absent();
    let change = model.apply_game(&add(key)).unwrap();
    let mut expected = keywords();
    expected.push(key);
    assert_eq!(keys(&mut model), expected);
    let after = model.document().to_json();
    assert_eq!(diff(&before, &after), [added(&format!("{LIST}[{}]", expected.len() - 1))]);
    // The inverse removes it again.
    replay(&mut model, &change.inverse).unwrap();
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn the_game_interface_takes_the_end_position_only() {
    let mut model = standard_model();
    let key = absent();
    let len = keywords().len();
    let before = model.document().to_json();
    for position in [0, 1, len - 1, len + 1] {
        let error = model.apply_game(&Edit::insert(row(key), Some(position), Vec::new())).unwrap_err();
        assert!(
            matches!(error, EditError::PositionOutOfRange { position: found, len: found_len, .. } if found == position && found_len == len),
            "{error:?}"
        );
        assert_eq!(model.document().to_json(), before, "position {position}");
    }
    model.apply_game(&Edit::insert(row(key), Some(len), Vec::new())).unwrap();
    assert_eq!(keys(&mut model).last(), Some(&key));
}

#[test]
fn the_refusals_name_their_errors_and_change_no_byte() {
    let repeated = keywords()[1];
    let weapon = ids(TableKind::Weapon)[0];
    let key = absent();
    let value_row = Edit::insert(row(key), None, vec![("count", Value::Int(1))]);
    let cases: [(&str, Edit); 4] = [("repeated", add(repeated)), ("weapon", add(weapon)), ("zero", add(0)), ("row value", value_row)];
    for (label, edit) in cases {
        for how in ["game", "enforce"] {
            let mut model = standard_model();
            let before = model.document().to_json();
            let error = match how {
                "game" => model.apply_game(&edit).unwrap_err(),
                _ => model.apply(&edit, CapPolicy::Enforce).unwrap_err(),
            };
            let named = match label {
                "repeated" => matches!(error, EditError::DuplicateRow { .. }),
                "weapon" => matches!(error, EditError::UnknownId { id, .. } if id == i64::from(weapon)),
                "zero" => matches!(error, EditError::OutOfRange { value: 0, min: 1, .. }),
                _ => matches!(error, EditError::UnknownField { .. }),
            };
            assert!(named, "{label} through {how}: {error:?}");
            assert_eq!(model.document().to_json(), before, "{label} through {how}");
        }
    }
}

#[test]
fn the_raw_interface_takes_a_key_the_table_lacks_only_in_storage_only() {
    let weapon = ids(TableKind::Weapon)[0];
    let mut model = standard_model();
    let before = model.document().to_json();
    let change = model.apply(&add(weapon), CapPolicy::StorageOnly).unwrap();
    assert_eq!(keys(&mut model).last(), Some(&weapon));
    model.apply(&change, CapPolicy::StorageOnly).unwrap();
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn the_raw_interface_checks_the_position_and_the_duplicate_under_each_policy() {
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        let mut model = standard_model();
        let before = model.document().to_json();
        let past = Edit::insert(row(absent()), Some(keywords().len() + 1), Vec::new());
        let error = model.apply(&past, policy).unwrap_err();
        assert!(matches!(error, EditError::PositionOutOfRange { .. }), "{error:?}");
        let error = model.apply(&add(keywords()[0]), policy).unwrap_err();
        assert!(matches!(error, EditError::DuplicateRow { .. }), "{error:?}");
        assert_eq!(model.document().to_json(), before);
    }
}

#[test]
fn the_raw_interface_inserts_at_a_position() {
    let mut model = standard_model();
    let key = absent();
    let change = model.apply(&Edit::insert(row(key), Some(1), Vec::new()), CapPolicy::Enforce).unwrap();
    let mut expected = keywords();
    expected.insert(1, key);
    assert_eq!(keys(&mut model), expected);
    assert!(matches!(change, Edit::Remove { .. }));
}

#[test]
fn a_game_interface_removal_is_read_only() {
    let mut model = standard_model();
    let before = model.document().to_json();
    for key in keywords() {
        let error = model.apply_game(&Edit::remove(row(key))).unwrap_err();
        assert!(matches!(error, EditError::ReadOnly { .. }), "{error:?}");
    }
    let error = model.apply_game(&Edit::remove(row(absent()))).unwrap_err();
    assert!(matches!(error, EditError::ReadOnly { .. }), "{error:?}");
    assert_eq!(model.document().to_json(), before);
}

#[test]
fn every_keyword_row_is_locked_in_the_game_interface() {
    let mut model = standard_model();
    for key in keywords() {
        assert!(model.row_locked(&row(key)).unwrap(), "keyword {key}");
    }
    let error = model.row_locked(&row(absent())).unwrap_err();
    assert!(matches!(error, EditError::MissingRow { .. }), "{error:?}");
}

#[test]
fn the_raw_removal_of_an_absent_key_is_a_missing_row() {
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        let mut model = standard_model();
        let error = model.apply(&Edit::remove(row(absent())), policy).unwrap_err();
        assert!(matches!(error, EditError::MissingRow { .. }), "{error:?}");
    }
}

#[test]
fn a_raw_removal_then_its_inverse_restores_the_order_and_the_bytes() {
    for policy in [CapPolicy::Enforce, CapPolicy::StorageOnly] {
        for key in keywords() {
            let mut model = standard_model();
            let before = model.document().to_json();
            let inverse = model.apply(&Edit::remove(row(key)), policy).unwrap();
            assert!(!keys(&mut model).contains(&key));
            assert_eq!(diff(&before, &model.document().to_json()), [removed(&format!("{LIST}[{}]", index_of(key)))]);
            model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
            assert_eq!(keys(&mut model), keywords());
            assert_eq!(model.document().to_json(), before, "keyword {key} under {policy:?}");
        }
    }
}

fn index_of(key: u32) -> usize {
    keywords().iter().position(|found| *found == key).unwrap()
}

#[test]
fn undo_of_a_removal_restores_the_index() {
    let mut model = standard_model();
    let middle = keywords()[1];
    let inverse = model.apply(&Edit::remove(row(middle)), CapPolicy::StorageOnly).unwrap();
    match &inverse {
        Edit::Insert { row: found, position, values } => {
            assert_eq!(found.key(), middle);
            assert_eq!(*position, Some(1));
            assert_eq!(values.len(), 0);
        }
        other => panic!("expected an Insert, got {other:?}"),
    }
    model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
    // Not at the end: the order is the original one.
    assert_eq!(keys(&mut model), keywords());
    assert_ne!(keys(&mut model).last(), Some(&middle));
}

#[test]
fn a_removal_leaves_the_other_keywords_in_their_order() {
    let mut model = standard_model();
    model.apply(&Edit::remove(row(keywords()[0])), CapPolicy::StorageOnly).unwrap();
    assert_eq!(keys(&mut model), keywords()[1..]);
}

#[test]
fn a_spaced_array_restores_its_content_with_the_layout_as_the_only_difference() {
    let mut model = spaced_model();
    let before = model.document().to_json();
    assert_eq!(keys(&mut model), keywords());
    let inverse = model.apply(&Edit::remove(row(keywords()[1])), CapPolicy::StorageOnly).unwrap();
    model.apply(&inverse, CapPolicy::StorageOnly).unwrap();
    assert_eq!(keys(&mut model), keywords());
    let changes = diff(&before, &model.document().to_json());
    assert_ne!(changes.len(), 0);
    assert!(changes.iter().all(|change| change.kind == Kind::Layout), "{changes:?}");
}

#[test]
fn an_empty_list_takes_a_keyword_at_position_zero() {
    let mut slot = SlotData::standard();
    slot.keywords = Vec::new();
    let mut model = model(&slot.render());
    let key = absent();
    model.apply(&Edit::insert(row(key), Some(0), Vec::new()), CapPolicy::Enforce).unwrap();
    assert_eq!(keys(&mut model), [key]);
}

#[test]
fn a_session_adds_undoes_and_redoes_a_keyword() {
    let mut history = History::open(&Ff2Format::new(), &SlotData::standard().render());
    let before = history.text();
    history.edit(&add(absent())).unwrap();
    let added_text = history.text();
    assert_ne!(added_text, before);
    assert!(history.undo());
    assert_eq!(history.text(), before);
    assert!(history.redo());
    assert_eq!(history.text(), added_text);
    // A refused removal records nothing.
    let error = history.edit(&Edit::remove(row(keywords()[0]))).unwrap_err();
    assert!(matches!(error, EditError::ReadOnly { .. }), "{error:?}");
    assert_eq!(history.undo.len(), 1);
}

#[test]
fn a_keyword_row_has_no_field() {
    let model = standard_model();
    let field = row(keywords()[0]).field(names::COUNT);
    assert!(model.field_meta(&field).is_none());
    assert!(matches!(model.field_locked(&field), Err(EditError::UnknownField { .. })));
    let mut model = standard_model();
    assert!(matches!(model.get(&field), Err(EditError::UnknownField { .. })));
}

#[test]
fn an_element_that_is_no_integer_is_a_format_error() {
    for text in ["[917, \"x\"]", "[917.5]", "[-3]"] {
        let mut document = Document::parse(&SlotData::standard().render()).unwrap();
        document.set_raw(&target(), text).unwrap();
        let mut model = Ff2Model::new(document, tables());
        let before = model.document().to_json();
        let error = model.rows(&collection()).unwrap_err();
        assert!(matches!(error, EditError::Format { .. }), "{text}: {error:?}");
        let error = model.apply(&add(absent()), CapPolicy::Enforce).unwrap_err();
        assert!(matches!(error, EditError::Format { .. }), "{text}: {error:?}");
        assert_eq!(model.document().to_json(), before, "{text}");
    }
}
