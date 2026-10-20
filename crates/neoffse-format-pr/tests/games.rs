//! The Pixel Remaster game table: its values, its pin to the format document
//! and the name rules that keep discovery inside `My Games`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::collections::HashSet;
use std::path::{Component, Path};

use neoffse_format_pr::games::GAMES;

const NUMERALS: [&str; 6] = ["I", "II", "III", "IV", "V", "VI"];

fn document() -> String {
    let path = neoffse_testkit::workspace_root().join("docs/formats/pixel-remaster.md");
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn the_table_holds_the_six_games_in_order() {
    let expected = [
        ("ff1-pr", 1_173_770, "FINAL FANTASY PR", "FINAL FANTASY.exe"),
        ("ff2-pr", 1_173_780, "FINAL FANTASY II PR", "FINAL FANTASY II.exe"),
        ("ff3-pr", 1_173_790, "FINAL FANTASY III PR", "FINAL FANTASY III.exe"),
        ("ff4-pr", 1_173_800, "FINAL FANTASY IV PR", "FINAL FANTASY IV.exe"),
        ("ff5-pr", 1_173_810, "FINAL FANTASY V PR", "FINAL FANTASY V.exe"),
        ("ff6-pr", 1_173_820, "FINAL FANTASY VI PR", "FINAL FANTASY VI.exe"),
    ];
    assert_eq!(GAMES.len(), expected.len());
    for (row, (id, app_id, folder, executable)) in GAMES.iter().zip(expected) {
        assert_eq!(row.id, id, "row {id}: id");
        assert_eq!(row.app_id, app_id, "row {id}: app_id");
        assert_eq!(row.folder, folder, "row {id}: folder");
        assert_eq!(row.executable, executable, "row {id}: executable");
    }
}

const HEADER: [&str; 4] = ["Game", "Steam app ID", "Game folder under My Games", "Executable"];

/// The cells of every line of the Games table, header and separator
/// included, backticks stripped.
fn table_lines(document: &str) -> Vec<Vec<String>> {
    document
        .lines()
        .skip_while(|line| line.trim_end() != "### Games")
        .skip(1)
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .map(|line| line.trim().trim_matches('|').split('|').map(|cell| cell.trim().replace('`', "")).collect())
        .collect()
}

/// The data rows of the Games table: no header, no separator.
fn table_rows(document: &str) -> Vec<Vec<String>> {
    table_lines(document).into_iter().skip(2).collect()
}

/// One message per disagreement between the document table and `GAMES`,
/// naming the row and the field.
fn mismatches(document: &str) -> Vec<String> {
    let lines = table_lines(document);
    let rows = table_rows(document);
    let mut found = Vec::new();
    if lines.first().map(Vec::as_slice) != Some(&HEADER.map(String::from)[..]) {
        found.push(format!("the header is {:?}, expected {HEADER:?}", lines.first()));
    }
    if rows.len() != GAMES.len() {
        found.push(format!("the table has {} rows, expected {}", rows.len(), GAMES.len()));
    }
    for ((row, game), numeral) in rows.iter().zip(GAMES).zip(NUMERALS) {
        let name = format!("FF {numeral} PR");
        if row.len() != HEADER.len() {
            found.push(format!("row {name}: {} cells, expected {}", row.len(), HEADER.len()));
        }
        let cell = |index: usize| row.get(index).map_or("", String::as_str);
        if cell(0) != name {
            found.push(format!("row {name}: game cell is {:?}", cell(0)));
        }
        if cell(1).parse::<u32>().ok() != Some(game.app_id) {
            found.push(format!("row {name}: app ID cell {:?}, expected {}", cell(1), game.app_id));
        }
        if cell(2) != game.folder {
            found.push(format!("row {name}: folder cell {:?}, expected {:?}", cell(2), game.folder));
        }
        if cell(3) != game.executable {
            found.push(format!("row {name}: executable cell {:?}, expected {:?}", cell(3), game.executable));
        }
    }
    found
}

#[test]
fn every_row_matches_the_games_table_of_the_format_document() {
    let document = document();
    assert_eq!(table_rows(&document).len(), 6, "the Games table must hold six rows");
    assert_eq!(mismatches(&document), Vec::<String>::new());
}

#[test]
fn a_changed_document_row_is_detected() {
    let document = document();

    let changed_id = document.replacen("1173790", "1173791", 1);
    assert_ne!(changed_id, document, "the app ID of FF III must appear in the document");
    let found = mismatches(&changed_id);
    assert!(found.iter().any(|message| message.contains("FF III PR")), "{found:?}");

    let heading = document.find("### Games").unwrap();
    let (head, table) = document.split_at(heading);
    let changed_folder = format!("{head}{}", table.replacen("`FINAL FANTASY V PR`", "`FINAL FANTASY V  PR`", 1));
    assert_ne!(changed_folder, document, "the folder of FF V must appear after the heading");
    let found = mismatches(&changed_folder);
    assert!(found.iter().any(|message| message.contains("FF V PR")), "{found:?}");

    let extra_cell = format!("{head}{}", table.replacen("`FINAL FANTASY III.exe` |", "`FINAL FANTASY III.exe` | extra |", 1));
    assert_ne!(extra_cell, document, "the executable of FF III must appear after the heading");
    let found = mismatches(&extra_cell);
    assert!(found.iter().any(|message| message.contains("FF III PR")), "{found:?}");

    let renamed_label = format!("{head}{}", table.replacen("| Executable |", "| Program |", 1));
    assert_ne!(renamed_label, document, "the Executable label must appear after the heading");
    let found = mismatches(&renamed_label);
    assert!(found.iter().any(|message| message.contains("header")), "{found:?}");
}

#[test]
fn ids_follow_the_game_id_rule() {
    for (index, game) in GAMES.iter().enumerate() {
        assert_ne!(game.id, "");
        assert!(
            game.id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
            "id {:?} breaks the [a-z0-9-] rule",
            game.id
        );
        assert_eq!(game.id, format!("ff{}-pr", index + 1));
    }
}

#[test]
fn ids_and_app_ids_are_unique() {
    let ids: HashSet<_> = GAMES.iter().map(|g| g.id).collect();
    let app_ids: HashSet<_> = GAMES.iter().map(|g| g.app_id).collect();
    let folders: HashSet<_> = GAMES.iter().map(|g| g.folder).collect();
    let executables: HashSet<_> = GAMES.iter().map(|g| g.executable).collect();
    assert_eq!(ids.len(), GAMES.len(), "duplicate id");
    assert_eq!(app_ids.len(), GAMES.len(), "duplicate app ID");
    assert_eq!(folders.len(), GAMES.len(), "duplicate folder");
    assert_eq!(executables.len(), GAMES.len(), "duplicate executable");
}

#[test]
fn folders_are_plain_single_names() {
    for game in GAMES {
        let folder = game.folder;
        assert_ne!(folder, "");
        assert!(!folder.contains(['/', '\\', ':']), "{folder:?} holds a separator");
        assert!(folder != "." && folder != "..", "{folder:?} is a relative marker");
        assert!(
            !folder.starts_with([' ', '.']) && !folder.ends_with([' ', '.']),
            "{folder:?} has an edge space or dot"
        );
        let components: Vec<_> = Path::new(folder).components().collect();
        assert!(
            matches!(components.as_slice(), [Component::Normal(_)]),
            "{folder:?} is not one normal component"
        );
    }
}

#[test]
fn the_executable_is_the_folder_without_pr_plus_exe() {
    for game in GAMES {
        let stem = game.folder.strip_suffix(" PR").unwrap();
        assert_eq!(format!("{stem}.exe"), game.executable, "row {}", game.id);
    }
}

#[test]
fn executables_are_plain_exe_names() {
    for game in GAMES {
        let name = game.executable;
        let stem = name.strip_suffix(".exe");
        assert!(stem.is_some(), "{name:?} lacks the lowercase .exe extension");
        assert_ne!(stem, Some(""), "{name:?} has no stem");
        assert!(!name.contains(['/', '\\']), "{name:?} holds a separator");
    }
}
