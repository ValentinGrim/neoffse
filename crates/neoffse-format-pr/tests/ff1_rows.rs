//! The raw `apply` for `Insert` and `Remove`: row order, bytes and inverses.
//! The raw interface holds no game rule, so a placeholder row and a worn row
//! go like any other.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::ff1_save::{SlotData, model, standard_model};
use neoffse_core::{CapPolicy, ChangeSet, Edit, EditError, FieldId, RowId, Value};
use neoffse_format_pr::document::Segment;
use neoffse_format_pr::ff1::{Ff1Model, names};

const ENFORCE: CapPolicy = CapPolicy::Enforce;
const STORAGE_ONLY: CapPolicy = CapPolicy::StorageOnly;

fn inventory() -> FieldId {
    FieldId::new(names::INVENTORY)
}

fn item(key: u32) -> RowId {
    RowId::new(inventory(), key)
}

fn key_item(key: u32) -> RowId {
    RowId::new(FieldId::new(names::KEY_ITEMS), key)
}

fn rows(model: &mut Ff1Model) -> Vec<u32> {
    model.rows(&inventory()).unwrap()
}

fn count_of(model: &mut Ff1Model, key: u32) -> Value {
    model.get(&inventory().with_id(key).with_name(names::COUNT)).unwrap()
}

fn raw(model: &Ff1Model, path: &[Segment<'_>]) -> String {
    model.document().clone().raw(path).unwrap().into_owned()
}

/// Path to the element `index` of `normalOwnedItemList.target`.
fn row_path(index: usize) -> [Segment<'static>; 6] {
    [
        Segment::Key("userData"),
        Segment::Doc,
        Segment::Key("normalOwnedItemList"),
        Segment::Doc,
        Segment::Key("target"),
        Segment::Index(index),
    ]
}

#[test]
fn insert_appends_a_row_with_the_initial_count() {
    let mut model = standard_model();
    let old = rows(&mut model);
    let inverse = model.apply(&Edit::insert(item(7), None, Vec::new()), ENFORCE).unwrap();
    assert_eq!(inverse, Edit::remove(item(7)));
    let now = rows(&mut model);
    assert_eq!(now.len(), old.len() + 1);
    assert_eq!(now.last(), Some(&7));
    assert_eq!(count_of(&mut model, 7), Value::Int(1));
    assert_eq!(raw(&model, &row_path(old.len())), r#""{\"contentId\":7,\"count\":1}""#);
}

#[test]
fn insert_at_a_position() {
    let len = rows(&mut standard_model()).len();
    for position in [0, 10, len] {
        let mut model = standard_model();
        let mut want = rows(&mut model);
        model.apply(&Edit::insert(item(7), Some(position), Vec::new()), ENFORCE).unwrap();
        want.insert(position, 7);
        assert_eq!(rows(&mut model), want, "position {position}");
    }
}

#[test]
fn insert_with_a_count() {
    let mut model = standard_model();
    let values = vec![(names::COUNT, Value::Int(5))];
    model.apply(&Edit::insert(item(7), None, values), ENFORCE).unwrap();
    assert_eq!(count_of(&mut model, 7), Value::Int(5));
}

#[test]
fn remove_returns_an_insert_at_the_old_index() {
    let mut model = standard_model();
    let index = rows(&mut model).iter().position(|key| *key == 2).unwrap();
    let inverse = model.apply(&Edit::remove(item(2)), ENFORCE).unwrap();
    assert_eq!(inverse, Edit::insert(item(2), Some(index), vec![(names::COUNT, Value::Int(7))]));
    assert!(!rows(&mut model).contains(&2));
}

#[test]
fn undo_restores_order_and_bytes() {
    let original = standard_model().document().to_json();
    let old = rows(&mut standard_model());
    let len = old.len();
    let edits = [
        Edit::insert(item(7), Some(0), Vec::new()),
        Edit::insert(item(7), Some(10), Vec::new()),
        Edit::insert(item(7), None, Vec::new()),
        Edit::insert(item(7), Some(len), vec![(names::COUNT, Value::Int(9))]),
        Edit::remove(item(100)),
        Edit::remove(item(17)),
        Edit::remove(item(165)),
    ];
    for edit in edits {
        let mut model = standard_model();
        let inverse = model.apply(&edit, ENFORCE).unwrap();
        assert_ne!(model.document().to_json(), original, "{edit:?}");
        let redo = model.apply(&inverse, STORAGE_ONLY).unwrap();
        assert_eq!(rows(&mut model), old, "{edit:?}");
        assert_eq!(model.document().to_json(), original, "{edit:?}");
        model.apply(&redo, STORAGE_ONLY).unwrap();
        model.apply(&inverse, STORAGE_ONLY).unwrap();
        assert_eq!(model.document().to_json(), original, "{edit:?}");
    }
}

#[test]
fn key_items_follow_the_same_rules() {
    let original = standard_model().document().to_json();
    let mut model = standard_model();
    let inverse = model.apply(&Edit::insert(key_item(47), None, Vec::new()), ENFORCE).unwrap();
    assert_eq!(inverse, Edit::remove(key_item(47)));
    let keys = model.rows(&FieldId::new(names::KEY_ITEMS)).unwrap();
    assert_eq!(keys.last(), Some(&47));
    model.apply(&inverse, STORAGE_ONLY).unwrap();
    assert_eq!(model.document().to_json(), original);

    let inverse = model.apply(&Edit::remove(key_item(46)), ENFORCE).unwrap();
    assert!(!model.rows(&FieldId::new(names::KEY_ITEMS)).unwrap().contains(&46));
    model.apply(&inverse, STORAGE_ONLY).unwrap();
    assert_eq!(model.document().to_json(), original);
}

#[test]
fn raw_remove_carries_no_game_rule() {
    let original = standard_model().document().to_json();
    let old = rows(&mut standard_model());
    for key in [62, 89] {
        let mut model = standard_model();
        let inverse = model.apply(&Edit::remove(item(key)), ENFORCE).unwrap();
        assert!(!rows(&mut model).contains(&key));
        model.apply(&inverse, STORAGE_ONLY).unwrap();
        assert_eq!(rows(&mut model), old, "{key}");
        assert_eq!(model.document().to_json(), original, "{key}");
    }
    let mut data = SlotData::standard();
    data.inventory.retain(|row| !row.text.contains("\"contentId\":62,"));
    let mut model = model(&data.render());
    let inverse = model.apply(&Edit::insert(item(62), None, Vec::new()), ENFORCE).unwrap();
    assert_eq!(rows(&mut model).last(), Some(&62));
    assert_eq!(inverse, Edit::remove(item(62)));
}

#[test]
fn a_change_set_with_rows_is_all_or_none() {
    let original = standard_model().document().to_json();
    let mut model = standard_model();
    let bad: ChangeSet = [Edit::insert(item(7), None, Vec::new()), Edit::insert(item(999), None, Vec::new())]
        .into_iter()
        .collect();
    let result = bad.apply_with(ENFORCE, |edit, policy| model.apply(edit, policy));
    assert!(matches!(result, Err(EditError::UnknownId { id: 999, .. })));
    assert_eq!(model.document().to_json(), original);

    let old = rows(&mut model);
    let good: ChangeSet = [Edit::insert(item(7), Some(3), Vec::new()), Edit::remove(item(17))].into_iter().collect();
    let inverse = good.apply_with(ENFORCE, |edit, policy| model.apply(edit, policy)).unwrap();
    assert_ne!(rows(&mut model), old);
    inverse.apply_with(STORAGE_ONLY, |edit, policy| model.apply(edit, policy)).unwrap();
    assert_eq!(rows(&mut model), old);
    assert_eq!(model.document().to_json(), original);
}

#[test]
fn insert_and_remove_leave_the_sort_list_and_other_lists_alone() {
    let mut data = SlotData::standard();
    data.sort_ids = vec![2, 3, 5];
    let mut model = model(&data.render());
    let user = |key: &'static str| [Segment::Key("userData"), Segment::Doc, Segment::Key(key)];
    let kept = ["normalOwnedItemSortIdList", "corpsList", "ownedCharacterList", "learnedAbilityList"];
    let before: Vec<String> = kept.iter().map(|key| raw(&model, &user(key))).collect();
    model.apply(&Edit::insert(item(7), None, Vec::new()), ENFORCE).unwrap();
    model.apply(&Edit::remove(item(100)), ENFORCE).unwrap();
    model.apply(&Edit::remove(item(89)), ENFORCE).unwrap();
    let after: Vec<String> = kept.iter().map(|key| raw(&model, &user(key))).collect();
    assert_eq!(before, after);
}

#[test]
fn a_removed_row_does_not_touch_the_siblings_text() {
    let mut model = standard_model();
    let index = rows(&mut model).iter().position(|key| *key == 17).unwrap();
    let before = raw(&model, &row_path(index - 1));
    let after_row = raw(&model, &row_path(index + 1));
    model.apply(&Edit::remove(item(17)), ENFORCE).unwrap();
    assert_eq!(raw(&model, &row_path(index - 1)), before);
    assert_eq!(raw(&model, &row_path(index)), after_row);
}
