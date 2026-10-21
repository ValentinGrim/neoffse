//! `SaveView` lookups, `SaveSummary` and `SaveSlot`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::time::Duration;

use neoffse_core::{Capability, CollectionMeta, Entity, FieldId, FieldMeta, FieldView, Game, RowId, SaveSlot, SaveSummary, SaveView, Section, Shape, Value};

fn field(id: FieldId, label: &str, value: i64) -> FieldView {
    FieldView::new(id, FieldMeta::new(label, Shape::Int { min: 0, max: 99 }), Value::Int(value))
}

fn character(n: u32) -> Entity {
    let id = FieldId::new("character").with_id(n);
    let fields = vec![
        field(id.clone().with_name("hp"), "HP", 10 + i64::from(n)),
        field(id.clone().with_name("mp"), "MP", 5),
    ];
    Entity::new(id, "Hero", fields)
}

fn inventory() -> FieldId {
    FieldId::new("inventory")
}

fn row_entity(key: u32, count: i64) -> Entity {
    let row = RowId::new(inventory(), key);
    Entity::new(row.id().clone(), "Item", vec![field(row.field("count"), "Count", count)])
}

fn view() -> SaveView {
    let party = Section::new(Capability::Party, vec![character(1), character(2)]);
    let mut stock = Section::new(Capability::Inventory, vec![row_entity(3, 1), row_entity(7, 2)]);
    stock.collections.push(CollectionMeta::new(
        inventory(),
        "Inventory",
        FieldMeta::new("Item", Shape::Int { min: 1, max: 200 }),
    ));
    SaveView::new(vec![party, stock])
}

#[test]
fn section_finds_by_capability() {
    let view = view();
    assert_eq!(view.section(Capability::Party).unwrap().capability, Capability::Party);
    assert_eq!(view.section(Capability::Inventory).unwrap().capability, Capability::Inventory);
    assert!(view.section(Capability::Magic).is_none());
}

#[test]
fn find_returns_the_entity_and_the_field() {
    let view = view();
    let id = RowId::new(inventory(), 7).field("count");
    let (entity, found) = view.find(&id).unwrap();
    assert_eq!(found.id, id);
    assert_eq!(found.value, Value::Int(2));
    assert_eq!(entity.id, RowId::new(inventory(), 7).id().clone());
    let hp = FieldId::new("character").with_id(2).with_name("hp");
    let (entity, found) = view.find(&hp).unwrap();
    assert_eq!((entity.id.clone(), found.value.clone()), (FieldId::new("character").with_id(2), Value::Int(12)));
}

#[test]
fn find_returns_none_for_an_unknown_id_and_for_a_prefix() {
    let view = view();
    assert!(view.find(&FieldId::new("nowhere")).is_none());
    assert!(view.find(&FieldId::new("character").with_id(1)).is_none());
    assert!(view.find(&FieldId::new("character").with_id(1).with_name("lv")).is_none());
}

#[test]
fn find_matches_ids_not_labels() {
    let view = view();
    for n in [1, 2] {
        let id = FieldId::new("character").with_id(n).with_name("mp");
        let (entity, found) = view.find(&id).unwrap();
        assert_eq!(found.id, id);
        assert_eq!(entity.id, FieldId::new("character").with_id(n));
        assert_eq!(found.meta.label, "MP");
    }
}

#[test]
fn collection_finds_the_metadata_by_collection_id() {
    let party = Section::new(Capability::Party, vec![]);
    assert_eq!(party.collections, vec![]);
    let view = view();
    assert_eq!(view.collection(&inventory()).unwrap().label, "Inventory");
    assert!(view.collection(RowId::new(inventory(), 3).id()).is_none());
    assert!(view.collection(&RowId::new(inventory(), 3).field("count")).is_none());
    assert!(view.collection(&FieldId::new("character")).is_none());
    let mut two = view.clone();
    two.sections[0].collections.push(CollectionMeta::new(
        FieldId::new("character"),
        "Characters",
        FieldMeta::new("Id", Shape::Int { min: 1, max: 2 }),
    ));
    assert_eq!(two.collection(&FieldId::new("character")).unwrap().label, "Characters");
    assert_eq!(two.collection(&inventory()).unwrap().label, "Inventory");
}

#[test]
fn lookups_return_the_first_match_in_view_order() {
    let early = FieldId::new("early");
    let later = FieldId::new("later");
    let mut first = Section::new(Capability::Party, vec![Entity::new(early.clone(), "First", vec![])]);
    let mut second = Section::new(Capability::Party, vec![Entity::new(later.clone(), "Second", vec![])]);
    let shared = FieldId::new("shared").with_name("hp");
    first.entities[0].fields = vec![field(shared.clone(), "One", 1), field(shared.clone(), "Two", 2)];
    second.entities[0].fields = vec![field(shared.clone(), "Three", 3)];
    let collection = |label: &str| CollectionMeta::new(inventory(), label, FieldMeta::new("Item", Shape::Int { min: 1, max: 200 }));
    first.collections = vec![collection("Early one"), collection("Early two")];
    second.collections = vec![collection("Later")];
    let view = SaveView::new(vec![first, second]);

    let party = view.section(Capability::Party).unwrap();
    assert_eq!(party.entities[0].id, early);
    let (entity, found) = view.find(&shared).unwrap();
    assert_eq!((entity.id.clone(), found.meta.label.as_str()), (early, "One"));
    assert_eq!(view.collection(&inventory()).unwrap().label, "Early one");

    let swapped = SaveView::new(view.sections.iter().rev().cloned().collect());
    assert_eq!(swapped.section(Capability::Party).unwrap().entities[0].id, later);
    assert_eq!(swapped.find(&shared).unwrap().1.meta.label, "Three");
    assert_eq!(swapped.collection(&inventory()).unwrap().label, "Later");
}

#[test]
fn a_view_equals_its_clone_and_differs_by_one_value() {
    let view = view();
    assert_eq!(view, view.clone());
    let mut changed = view.clone();
    changed.sections[1].entities[1].fields[0].value = Value::Int(3);
    assert_ne!(view, changed);
}

#[test]
fn summary_new_holds_the_game_alone() {
    let mut summary = SaveSummary::new(Game::Ff1Pr);
    assert_eq!(summary.game, Game::Ff1Pr);
    assert!(summary.slot.is_none() && summary.play_time.is_none() && summary.location.is_none());
    assert_eq!(summary.party, Vec::<String>::new());
    summary.slot = Some(SaveSlot::Quick);
    summary.play_time = Some(Duration::from_secs(60));
    summary.location = Some("Lakeside".into());
    summary.party = vec!["Ada".into()];
    assert_eq!(summary.slot, Some(SaveSlot::Quick));
    assert_eq!(summary.play_time, Some(Duration::from_secs(60)));
    assert_eq!(summary.location.as_deref(), Some("Lakeside"));
    assert_eq!(summary.party, ["Ada"]);
}

#[test]
fn slots_compare_by_variant_and_number() {
    assert_ne!(SaveSlot::Numbered(1), SaveSlot::Numbered(2));
    assert_ne!(SaveSlot::Numbered(1), SaveSlot::Auto);
    assert_ne!(SaveSlot::Auto, SaveSlot::Quick);
    assert_eq!(SaveSlot::Numbered(1), SaveSlot::Numbered(1));
}

fn assert_send_sync_static<T: Send + Sync + 'static>() {}

#[test]
fn view_types_are_send_sync_and_static() {
    assert_send_sync_static::<SaveView>();
    assert_send_sync_static::<Section>();
    assert_send_sync_static::<Entity>();
    assert_send_sync_static::<FieldView>();
    assert_send_sync_static::<SaveSummary>();
    assert_send_sync_static::<SaveSlot>();
}
