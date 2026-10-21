//! The slot-name table: its pin to the format document and the name rules.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_format_pr::slots::SLOT_FILES;

const HEADER: [&str; 4] = ["Slot", "PC file name", "id", "PlayStation file"];

fn document() -> String {
    let path = neoffse_testkit::workspace_root().join("docs/formats/pixel-remaster.md");
    std::fs::read_to_string(path).unwrap()
}

/// The cells of every line of the Slot files table, header and separator
/// included, backticks stripped.
fn table_lines(document: &str) -> Vec<Vec<String>> {
    document
        .lines()
        .skip_while(|line| line.trim_end() != "### Slot files")
        .skip(1)
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .map(|line| line.trim().trim_matches('|').split('|').map(|cell| cell.trim().replace('`', "")).collect())
        .collect()
}

/// The row labels in table order: `Auto Save`, `Quick Save`, `1` to `20`.
fn labels() -> Vec<String> {
    ["Auto Save", "Quick Save"]
        .into_iter()
        .map(String::from)
        .chain((1..=20).map(|n| n.to_string()))
        .collect()
}

/// One message per disagreement between the document table and
/// `SLOT_FILES`, naming the row and the field.
fn mismatches(document: &str) -> Vec<String> {
    let lines = table_lines(document);
    let mut found = Vec::new();
    if lines.first().map(Vec::as_slice) != Some(&HEADER.map(String::from)[..]) {
        found.push(format!("the header is {:?}, expected {HEADER:?}", lines.first()));
    }
    let rows: Vec<_> = lines.iter().skip(2).collect();
    if rows.len() != SLOT_FILES.len() {
        found.push(format!("the table has {} rows, expected {}", rows.len(), SLOT_FILES.len()));
    }
    for ((row, file), label) in rows.iter().zip(SLOT_FILES).zip(labels()) {
        if row.len() != HEADER.len() {
            found.push(format!("row {label}: {} cells, expected {}", row.len(), HEADER.len()));
        }
        let cell = |index: usize| row.get(index).map_or("", String::as_str);
        if cell(0) != label {
            found.push(format!("row {label}: slot cell is {:?}", cell(0)));
        }
        if cell(1) != file.name {
            found.push(format!("row {label}: name cell {:?}, expected {:?}", cell(1), file.name));
        }
        if cell(2).parse::<u32>().ok() != Some(file.id) {
            found.push(format!("row {label}: id cell {:?}, expected {}", cell(2), file.id));
        }
        if cell(3) != format!("slot{}.sav", file.id) {
            found.push(format!("row {label}: PlayStation cell {:?}, expected slot{}.sav", cell(3), file.id));
        }
    }
    found
}

#[test]
fn every_row_matches_the_slot_files_table_of_the_format_document() {
    let document = document();
    assert_eq!(
        table_lines(&document).len(),
        24,
        "the Slot files table must hold a header, a separator and 22 rows"
    );
    assert_eq!(mismatches(&document), Vec::<String>::new());
}

#[test]
fn a_changed_document_row_is_detected() {
    let document = document();

    let name = SLOT_FILES[4].name;
    let changed_name = document.replacen(name, &name.replacen('Q', "R", 1), 1);
    assert_ne!(changed_name, document, "the name of slot 3 must appear in the document");
    let found = mismatches(&changed_name);
    assert!(found.iter().any(|message| message.contains("row 3:")), "{found:?}");

    let heading = document.find("### Slot files").unwrap();
    let (head, table) = document.split_at(heading);
    let row = format!("`{}` | 14 |", SLOT_FILES[15].name);
    let changed_id = format!("{head}{}", table.replacen(&row, &row.replace("| 14 |", "| 41 |"), 1));
    assert_ne!(changed_id, document, "the id of slot 14 must appear after the heading");
    let found = mismatches(&changed_id);
    assert!(found.iter().any(|message| message.contains("row 14:")), "{found:?}");

    let renamed_label = format!("{head}{}", table.replacen("| PC file name |", "| File |", 1));
    assert_ne!(renamed_label, document, "the PC file name label must appear after the heading");
    let found = mismatches(&renamed_label);
    assert!(found.iter().any(|message| message.contains("header")), "{found:?}");
}

#[test]
fn names_are_44_characters_of_the_base64_alphabet() {
    for file in SLOT_FILES {
        let name = file.name;
        assert_eq!(name.len(), 44, "slot {}", file.id);
        assert!(
            name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=')),
            "slot {}: a byte outside the base64 alphabet",
            file.id
        );
        assert!(!name.contains(['/', '\\', ':']), "slot {}: a path separator", file.id);
    }
}
