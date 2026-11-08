//! The FF II keyword rows on the Items page: an add picker fed by the Keyword
//! table, a locked row with a disabled Remove in the default mode, an enabled
//! Remove in expert mode. The page code names no keyword: the rows come from
//! the collection metadata. No window shows.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{Editing, ff2_editing, ff2_site, ff2_ui, list, resize, rows};
use i_slint_backend_testing::AccessibleRole::Button;
use neoffse::core::{Game, TableKind};
use neoffse::data::tables_for;
use neoffse_app::{EditProblem, EditorCommand, ItemList, ReviewKind};

const KEYWORDS: [&str; 4] = ["Wild Rose", "Mythril", "Airship", "Dreadnought"];

fn keywords(ed: &Editing) -> ItemList {
    list(&ed.data(), "Keywords")
}

fn labels(list: &ItemList) -> Vec<String> {
    rows(list).iter().map(|row| row.label.to_string()).collect()
}

/// The id of a keyword the fixture lacks.
fn spare_keyword(ed: &Editing) -> (i32, String) {
    let held = labels(&keywords(ed));
    let tables = tables_for(Game::Ff2Pr).unwrap();
    let table = tables.table(TableKind::Keyword).unwrap();
    let entry = table
        .iter()
        .find(|entry| {
            !entry.placeholder
                && entry
                    .name(neoffse::data::Language::En)
                    .is_some_and(|name| !held.iter().any(|label| label == name))
        })
        .expect("a keyword the fixture lacks");
    (i32::try_from(entry.id).unwrap(), entry.name(neoffse::data::Language::En).unwrap().to_owned())
}

#[test]
fn the_items_page_lists_the_keywords() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let keywords = keywords(&ed);
    assert_eq!(labels(&keywords), KEYWORDS);
    assert!(keywords.insertable);
}

#[test]
fn the_add_picker_offers_the_keyword_table_without_placeholders() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let entries = ed.editor().list_entries(keywords(&ed).key, "");
    let tables = tables_for(Game::Ff2Pr).unwrap();
    let table = tables.table(TableKind::Keyword).unwrap();
    let expected = table.iter().filter(|entry| !entry.placeholder).count();
    assert_eq!(entries.len(), expected);
    assert_eq!(entries.len(), 15, "the Keyword table holds rows 917 to 931");
    assert!(entries.iter().all(|entry| (917..=931).contains(&entry.id)));
}

#[test]
fn default_mode_adds_a_keyword_and_locks_every_row() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let (id, name) = spare_keyword(&ed);

    assert!(ed.editor().add_row(keywords(&ed).key, id).is_none());
    assert_eq!(ed.pending(), 1);
    let after = keywords(&ed);
    let mut expected: Vec<String> = KEYWORDS.iter().map(ToString::to_string).collect();
    expected.push(name);
    assert_eq!(labels(&after), expected);
    for row in rows(&after) {
        assert!(row.removable, "{}", row.label);
        assert!(row.locked, "{} shows no lock mark", row.label);
        assert!(!row.can_remove, "{} has an enabled Remove", row.label);
    }
}

#[test]
fn default_mode_refuses_a_removal() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let key = rows(&keywords(&ed))[1].key;
    let notice = ed.editor().remove_row(key).expect("a refusal");
    assert_eq!(notice.problem, EditProblem::ReadOnly);
    assert_eq!(ed.pending(), 0);
    assert_eq!(labels(&keywords(&ed)), KEYWORDS);
}

#[test]
fn expert_mode_enables_remove_and_undo_restores_the_keyword_in_place() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    ed.editor().set_expert(true);
    let rows_now = rows(&keywords(&ed));
    assert!(rows_now.iter().all(|row| row.locked && row.can_remove));

    assert!(ed.editor().remove_row(rows_now[1].key).is_none());
    assert_eq!(labels(&keywords(&ed)), ["Wild Rose", "Airship", "Dreadnought"]);
    assert_eq!(ed.pending(), 1);

    assert!(ed.editor().undo().is_none());
    assert_eq!(labels(&keywords(&ed)), KEYWORDS);
    assert_eq!(ed.pending(), 0);
}

#[test]
fn the_review_lists_an_added_and_a_removed_keyword() {
    let site = ff2_site();
    let ed = ff2_editing(&site);
    let (id, name) = spare_keyword(&ed);
    assert!(ed.editor().add_row(keywords(&ed).key, id).is_none());
    ed.editor().set_expert(true);
    let key = rows(&keywords(&ed))[0].key;
    assert!(ed.editor().remove_row(key).is_none());

    // Default mode lists both: a keyword row is no expert-only page.
    ed.editor().set_expert(false);
    let rows = ed.editor().review().unwrap();
    let found: Vec<(ReviewKind, String)> = rows.iter().map(|row| (row.kind, row.entity.to_string())).collect();
    assert!(found.contains(&(ReviewKind::Added, name)), "{found:?}");
    assert!(found.contains(&(ReviewKind::Removed, "Wild Rose".to_owned())), "{found:?}");
}

fn remove_enabled(ui: &common::Ui, label: &str) -> bool {
    ui.element(&format!("Remove {label}"), Button).accessible_enabled() == Some(true)
}

#[test]
fn the_remove_button_is_disabled_in_default_mode_and_enabled_in_expert_mode() {
    let site = ff2_site();
    let ui = ff2_ui(&site);
    // Tall enough for the 30 item rows above the keywords: a clipped row is absent from the tree.
    resize(&ui, 1280.0, 4000.0);
    ui.navigate(EditorCommand::Items, -1);
    for label in KEYWORDS {
        assert!(!remove_enabled(&ui, label), "{label}: Remove is enabled in the default mode");
    }

    ui.bridge().invoke_expert_toggle_requested();
    ui.bridge().invoke_expert_warning_confirmed();
    for label in KEYWORDS {
        assert!(remove_enabled(&ui, label), "{label}: Remove is disabled in expert mode");
    }
}
