//! `field_kind`: the widget a field's metadata asks for.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::core::{FieldMeta, Game, NamedValue, Shape, TableKind};
use neoffse::data::{GameTables, tables_for};
use neoffse_app::FieldKind;
use neoffse_app::editor::field_kind;

const INT: Shape = Shape::Int { min: 0, max: 100 };

fn tables() -> &'static GameTables {
    tables_for(Game::Ff1Pr).expect("the FF I tables are embedded")
}

fn int_with(tables_of: &[TableKind]) -> FieldMeta {
    let mut meta = FieldMeta::new("Value", INT);
    meta.tables = tables_of.to_vec();
    meta
}

fn named(value: i64, name: &str) -> NamedValue {
    NamedValue { value, name: name.to_owned() }
}

#[test]
fn a_read_only_field_is_a_label_even_with_tables() {
    let mut meta = int_with(&[TableKind::Ability]);
    meta.read_only = true;
    assert_eq!(field_kind(&meta, Some(tables())), FieldKind::Label);
    meta.names.push(named(0, "Empty"));
    assert_eq!(field_kind(&meta, Some(tables())), FieldKind::Label);
}

#[test]
fn bool_and_text_get_their_widgets() {
    let flag = FieldMeta::new("Flag", Shape::Bool);
    let text = FieldMeta::new("Name", Shape::Text);
    assert_eq!(field_kind(&flag, Some(tables())), FieldKind::Toggle);
    assert_eq!(field_kind(&text, Some(tables())), FieldKind::Text);
    assert_eq!(field_kind(&flag, None), FieldKind::Toggle);
    assert_eq!(field_kind(&text, None), FieldKind::Text);

    let mut locked_flag = flag;
    locked_flag.read_only = true;
    assert_eq!(field_kind(&locked_flag, Some(tables())), FieldKind::Label);
    let mut locked_text = text;
    locked_text.read_only = true;
    assert_eq!(field_kind(&locked_text, Some(tables())), FieldKind::Label);
}

#[test]
fn a_table_gives_a_picker_when_the_game_has_it() {
    let job = int_with(&[TableKind::Job]);
    assert_eq!(field_kind(&job, Some(tables())), FieldKind::Picker);
    assert_eq!(field_kind(&job, None), FieldKind::Number);

    let map = int_with(&[TableKind::Map]);
    assert!(tables().table(TableKind::Map).is_none(), "no game fills the Map table yet");
    assert_eq!(field_kind(&map, Some(tables())), FieldKind::Number);

    let mixed = int_with(&[TableKind::Map, TableKind::Job]);
    assert_eq!(field_kind(&mixed, Some(tables())), FieldKind::Picker);
}

#[test]
fn names_give_a_choice_and_tables_win_over_names() {
    let mut meta = FieldMeta::new("Mode", INT);
    meta.names = vec![named(0, "Off"), named(1, "On")];
    assert_eq!(field_kind(&meta, Some(tables())), FieldKind::Choice);
    assert_eq!(field_kind(&meta, None), FieldKind::Choice);

    meta.tables = vec![TableKind::Job];
    assert_eq!(field_kind(&meta, Some(tables())), FieldKind::Picker);
    assert_eq!(field_kind(&meta, None), FieldKind::Choice);
}

#[test]
fn a_plain_int_is_a_number() {
    let meta = FieldMeta::new("Level", INT);
    assert_eq!(field_kind(&meta, Some(tables())), FieldKind::Number);
    assert_eq!(field_kind(&meta, None), FieldKind::Number);
}
