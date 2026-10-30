//! `changes::diff` over hand-built views: no format, no session.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::changes::{Change, ChangeKind, diff};
use neoffse::core::{Capability, Entity, FieldId, FieldMeta, FieldView, Game, NamedValue, SaveView, Section, Shape, TableKind, Value};
use neoffse::data::{Language, tables_for};

fn entity_id(n: u32) -> FieldId {
    FieldId::new("thing").with_id(n)
}

fn field_id(n: u32, name: &'static str) -> FieldId {
    entity_id(n).with_name(name)
}

fn int_meta(label: &str) -> FieldMeta {
    FieldMeta::new(label, Shape::Int { min: -10, max: 1000 })
}

fn int_field(n: u32, name: &'static str, value: i64) -> FieldView {
    FieldView::new(field_id(n, name), int_meta(name), Value::Int(value))
}

fn entity(n: u32, label: &str, fields: Vec<FieldView>) -> Entity {
    Entity::new(entity_id(n), label, fields)
}

fn view(capability: Capability, entities: Vec<Entity>) -> SaveView {
    SaveView::new(vec![Section::new(capability, entities)])
}

fn one(fields: Vec<FieldView>) -> SaveView {
    view(Capability::Party, vec![entity(1, "Aria", fields)])
}

fn compare(before: &SaveView, after: &SaveView) -> Vec<Change> {
    diff(before, after, None, Language::En)
}

fn field_change(n: u32, name: &'static str, label: &str, before: &str, after: &str) -> ChangeKind {
    ChangeKind::Field {
        field: field_id(n, name),
        label: label.to_owned(),
        before: before.to_owned(),
        after: after.to_owned(),
    }
}

#[test]
fn equal_views_give_no_change() {
    let view = one(vec![int_field(1, "level", 5)]);
    assert_eq!(compare(&view, &view.clone()), Vec::<Change>::new());
    assert_eq!(compare(&SaveView::new(Vec::new()), &SaveView::new(Vec::new())), Vec::<Change>::new());
}

#[test]
fn text_int_and_bool_changes_show_both_sides() {
    let text = |value: &str| FieldView::new(field_id(1, "name"), FieldMeta::new("Name", Shape::Text), Value::Text(value.to_owned()));
    let flag = |value: bool| FieldView::new(field_id(1, "flag"), FieldMeta::new("Flag", Shape::Bool), Value::Bool(value));
    let before = one(vec![text("Aria"), int_field(1, "level", 5), flag(true)]);
    let after = one(vec![text("Zed"), int_field(1, "level", 6), flag(false)]);
    assert_eq!(
        compare(&before, &after),
        [
            Change::new(Capability::Party, entity_id(1), "Aria", field_change(1, "name", "Name", "Aria", "Zed")),
            Change::new(Capability::Party, entity_id(1), "Aria", field_change(1, "level", "level", "5", "6")),
            Change::new(Capability::Party, entity_id(1), "Aria", field_change(1, "flag", "Flag", "Yes", "No")),
        ]
    );
}

#[test]
fn a_named_value_shows_its_name() {
    let named = |value: i64| {
        let mut meta = int_meta("Job");
        meta.names = vec![
            NamedValue {
                value: 1,
                name: "Knight".to_owned(),
            },
            NamedValue {
                value: 2,
                name: "Ninja".to_owned(),
            },
        ];
        FieldView::new(field_id(1, "job"), meta, Value::Int(value))
    };
    let changes = compare(&one(vec![named(1)]), &one(vec![named(2)]));
    assert_eq!(changes[0].kind, field_change(1, "job", "Job", "Knight", "Ninja"));
    let changes = compare(&one(vec![named(1)]), &one(vec![named(9)]));
    assert_eq!(changes[0].kind, field_change(1, "job", "Job", "Knight", "9"));
}

#[test]
fn read_only_fields_are_skipped() {
    let derived = |value: i64| {
        let mut meta = int_meta("Max HP");
        meta.read_only = true;
        FieldView::new(field_id(1, "max_hp"), meta, Value::Int(value))
    };
    let before = one(vec![derived(10), int_field(1, "level", 5)]);
    let only_derived = one(vec![derived(20), int_field(1, "level", 5)]);
    assert_eq!(compare(&before, &only_derived), Vec::<Change>::new());

    let both = one(vec![derived(20), int_field(1, "level", 6)]);
    assert_eq!(
        compare(&before, &both),
        [Change::new(
            Capability::Party,
            entity_id(1),
            "Aria",
            field_change(1, "level", "level", "5", "6")
        )]
    );
}

#[test]
fn a_row_on_one_side_is_added_or_removed() {
    let before = view(
        Capability::Inventory,
        vec![
            entity(1, "Kept", vec![int_field(1, "count", 1)]),
            entity(2, "Gone", vec![int_field(2, "count", 1)]),
        ],
    );
    let after = view(
        Capability::Inventory,
        vec![
            entity(1, "Kept", vec![int_field(1, "count", 2)]),
            entity(3, "New", vec![int_field(3, "count", 1)]),
        ],
    );
    assert_eq!(
        compare(&before, &after),
        [
            Change::new(Capability::Inventory, entity_id(1), "Kept", field_change(1, "count", "count", "1", "2")),
            Change::new(Capability::Inventory, entity_id(3), "New", ChangeKind::RowAdded),
            Change::new(Capability::Inventory, entity_id(2), "Gone", ChangeKind::RowRemoved),
        ]
    );
}

#[test]
fn a_moved_row_is_no_change() {
    let first = entity(1, "A", vec![int_field(1, "count", 1)]);
    let second = entity(2, "B", vec![int_field(2, "count", 2)]);
    let before = view(Capability::Inventory, vec![first.clone(), second.clone()]);
    let after = view(Capability::Inventory, vec![second, first]);
    assert_eq!(compare(&before, &after), Vec::<Change>::new());
}

#[test]
fn a_field_on_one_side_only_shows_an_empty_text() {
    let bare = one(vec![int_field(1, "level", 5)]);
    let extra = one(vec![int_field(1, "level", 5), int_field(1, "bonus", 3)]);
    assert_eq!(
        compare(&bare, &extra),
        [Change::new(Capability::Party, entity_id(1), "Aria", field_change(1, "bonus", "bonus", "", "3"))]
    );
    assert_eq!(
        compare(&extra, &bare),
        [Change::new(Capability::Party, entity_id(1), "Aria", field_change(1, "bonus", "bonus", "3", ""))]
    );
}

#[test]
fn sections_pair_by_capability() {
    let rows = |label: &str| vec![entity(1, label, Vec::new()), entity(2, label, Vec::new())];
    let party = Section::new(Capability::Party, rows("P"));
    let magic = Section::new(Capability::Magic, rows("M"));

    let before = SaveView::new(vec![party.clone(), magic.clone()]);
    let without = SaveView::new(vec![party.clone()]);
    assert_eq!(
        compare(&before, &without),
        [
            Change::new(Capability::Magic, entity_id(1), "M", ChangeKind::RowRemoved),
            Change::new(Capability::Magic, entity_id(2), "M", ChangeKind::RowRemoved),
        ]
    );
    assert_eq!(
        compare(&without, &before),
        [
            Change::new(Capability::Magic, entity_id(1), "M", ChangeKind::RowAdded),
            Change::new(Capability::Magic, entity_id(2), "M", ChangeKind::RowAdded),
        ]
    );
}

#[test]
fn the_order_follows_the_views() {
    let pair = |a: i64, b: i64, c: i64, d: i64| {
        view(
            Capability::Party,
            vec![
                entity(1, "One", vec![int_field(1, "x", a), int_field(1, "y", b)]),
                entity(2, "Two", vec![int_field(2, "x", c), int_field(2, "y", d)]),
            ],
        )
    };
    let changes = compare(&pair(1, 2, 3, 4), &pair(5, 6, 7, 8));
    let seen: Vec<(u32, &str)> = changes
        .iter()
        .map(|change| {
            let ChangeKind::Field { field, .. } = &change.kind else {
                panic!("a field change")
            };
            let id = if change.entity == entity_id(1) { 1 } else { 2 };
            (id, if *field == field_id(id, "x") { "x" } else { "y" })
        })
        .collect();
    assert_eq!(seen, [(1, "x"), (1, "y"), (2, "x"), (2, "y")]);
}

#[test]
fn tables_name_ids_in_the_chosen_language() {
    let tables = tables_for(Game::Ff1Pr);
    let with = |kind: TableKind, value: i64| {
        let mut meta = int_meta("Thing");
        meta.tables = vec![kind];
        one(vec![FieldView::new(field_id(1, "x"), meta, Value::Int(value))])
    };
    let texts = |before: &SaveView, after: &SaveView, language: Language| {
        let changes = diff(before, after, tables, language);
        let ChangeKind::Field { before, after, .. } = &changes[0].kind else {
            panic!("a field change")
        };
        (before.clone(), after.clone())
    };
    let (job7, job8) = (with(TableKind::Job, 7), with(TableKind::Job, 8));
    assert_eq!(texts(&job7, &job8, Language::En), ("Knight".to_owned(), "Ninja".to_owned()));
    assert_eq!(texts(&job7, &job8, Language::Fr), ("Paladin".to_owned(), "Ninja".to_owned()));
    assert_eq!(texts(&job7, &with(TableKind::Job, 99), Language::En).1, "99");
    assert_eq!(texts(&job7, &with(TableKind::Job, -1), Language::En).1, "-1");
    assert_eq!(
        texts(&with(TableKind::Weapon, 89), &with(TableKind::Weapon, 62), Language::En),
        ("Sun Blade".to_owned(), "Empty".to_owned())
    );

    let named = |value: i64| {
        let mut meta = int_meta("Job");
        meta.tables = vec![TableKind::Job];
        meta.names = vec![NamedValue {
            value: 7,
            name: "Chosen".to_owned(),
        }];
        one(vec![FieldView::new(field_id(1, "x"), meta, Value::Int(value))])
    };
    assert_eq!(texts(&named(7), &named(8), Language::En), ("Chosen".to_owned(), "Ninja".to_owned()));
}

#[test]
fn without_tables_an_id_shows_its_number() {
    let mut meta = int_meta("Job");
    meta.tables = vec![TableKind::Job];
    let make = |value: i64| one(vec![FieldView::new(field_id(1, "x"), meta.clone(), Value::Int(value))]);
    let changes = compare(&make(7), &make(8));
    assert_eq!(changes[0].kind, field_change(1, "x", "Job", "7", "8"));
}
