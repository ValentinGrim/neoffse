//! A listed row without a name reads `Entry <id>` in the review; an id no table lists keeps its number
//! (`docs/formats/pixel-remaster.md#rows-without-a-name`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::changes::{ChangeKind, diff};
use neoffse::core::{Capability, Entity, FieldId, FieldMeta, FieldView, Game, SaveView, Section, Shape, TableKind, Value};
use neoffse::data::{Language, tables_for};

/// Ability 711 is listed in FF II and carries no name.
const NAMELESS: i64 = 711;
/// Ability 1 is "Attack" in English and "Attaque" in French.
const NAMED: i64 = 1;
/// Past the last ability row.
const UNLISTED: i64 = 999_999;

fn view_of(value: i64) -> SaveView {
    let id = FieldId::new("thing").with_id(1);
    let mut meta = FieldMeta::new("Ability", Shape::Int { min: 0, max: 1_000_000 });
    meta.tables = vec![TableKind::Ability];
    let field = FieldView::new(id.clone().with_name("x"), meta, Value::Int(value));
    SaveView::new(vec![Section::new(Capability::Party, vec![Entity::new(id, "Aria", vec![field])])])
}

/// The clean and current texts of the one field change from `before` to `after`.
fn texts(before: i64, after: i64, language: Language) -> (String, String) {
    let tables = tables_for(Game::Ff2Pr);
    assert!(tables.is_some(), "FF II tables are embedded");
    let changes = diff(&view_of(before), &view_of(after), tables, language);
    let [change] = changes.as_slice() else {
        panic!("one change, found {}", changes.len())
    };
    let ChangeKind::Field { before, after, .. } = &change.kind else {
        panic!("a field change")
    };
    (before.clone(), after.clone())
}

#[test]
fn a_listed_nameless_row_reads_entry() {
    let tables = tables_for(Game::Ff2Pr).unwrap();
    let entry = tables.entry(TableKind::Ability, 711).unwrap();
    assert_eq!(entry.name(Language::En), None);
    assert_eq!(texts(NAMED, NAMELESS, Language::En), ("Attack".to_owned(), "Entry 711".to_owned()));
}

#[test]
fn the_clean_text_follows_the_same_rule() {
    assert_eq!(texts(NAMELESS, NAMED, Language::En), ("Entry 711".to_owned(), "Attack".to_owned()));
}

#[test]
fn the_label_does_not_depend_on_the_language() {
    assert_eq!(texts(NAMED, NAMELESS, Language::Fr), ("Attaque".to_owned(), "Entry 711".to_owned()));
}

#[test]
fn an_unlisted_id_keeps_its_number() {
    assert_eq!(texts(NAMED, UNLISTED, Language::En), ("Attack".to_owned(), "999999".to_owned()));
    assert_eq!(texts(UNLISTED, NAMELESS, Language::En), ("999999".to_owned(), "Entry 711".to_owned()));
}

#[test]
fn a_negative_value_keeps_its_number() {
    assert_eq!(texts(NAMED, -1, Language::En).1, "-1");
}
