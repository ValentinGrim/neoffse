//! Value rules of the file contract: one failing text per rule, and the
//! valid forms that must still parse.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{minimal, rule};
use neoffse_core::{Game, TableKind};
use neoffse_data::{DataError, GameTables, Sources};

fn parse(sources: &Sources<'_>) -> Result<GameTables, DataError> {
    GameTables::parse(Game::Ff1Pr, sources)
}

/// The file, row and rule of the `Rule` error the sources raise.
fn rule_of(sources: &Sources<'_>) -> (&'static str, String, &'static str) {
    rule(parse(sources))
}

fn ability(id: u32, content_id: u32, level: u32) -> String {
    format!("[[ability]]\nid = {id}\ncontent_id = {content_id}\nkind = \"white_magic\"\nlevel = {level}\nname_en = \"A\"\nname_fr = \"B\"\n")
}

fn content(id: u32, kind: &str, placeholder: &str) -> String {
    format!("[[content]]\nid = {id}\nkind = \"{kind}\"\n{placeholder}name_en = \"A\"\nname_fr = \"B\"\n\n")
}

#[test]
fn a_level_above_8_is_refused() {
    let text = ability(1, 209, 9);
    let sources = Sources { abilities: &text, ..minimal() };
    assert_eq!(rule_of(&sources), ("abilities.toml", "id 1".to_string(), "level above 8"));
    let text = ability(1, 209, 8);
    assert!(parse(&Sources { abilities: &text, ..minimal() }).is_ok());
}

#[test]
fn a_content_id_of_zero_is_refused() {
    let text = ability(1, 209, 1) + "\n" + &ability(2, 0, 1);
    let sources = Sources { abilities: &text, ..minimal() };
    assert_eq!(rule_of(&sources), ("abilities.toml", "id 2".to_string(), "content_id 0"));
}

#[test]
fn a_placeholder_written_as_false_is_refused() {
    let text = content(5, "weapon", "placeholder = false\n");
    let sources = Sources { content: &text, ..minimal() };
    assert_eq!(rule_of(&sources), ("content.toml", "id 5".to_string(), "placeholder written as false"));
}

#[test]
fn a_placeholder_off_equipment_is_refused() {
    for kind in ["item", "key_item"] {
        let text = content(5, kind, "placeholder = true\n");
        let sources = Sources { content: &text, ..minimal() };
        assert_eq!(
            rule_of(&sources),
            ("content.toml", "id 5".to_string(), "placeholder on a non-equipment kind"),
            "{kind}"
        );
    }
}

#[test]
fn a_second_placeholder_of_a_kind_is_refused() {
    let text = content(8, "weapon", "placeholder = true\n") + &content(9, "weapon", "placeholder = true\n");
    let sources = Sources { content: &text, ..minimal() };
    assert_eq!(rule_of(&sources), ("content.toml", "id 9".to_string(), "second placeholder of a kind"));
}

#[test]
fn one_placeholder_per_equipment_kind_parses() {
    let kinds = [
        ("weapon", TableKind::Weapon),
        ("shield", TableKind::Shield),
        ("armor", TableKind::Armor),
        ("helmet", TableKind::Helmet),
        ("gloves", TableKind::Gloves),
    ];
    let mut text = String::new();
    for (index, (name, _)) in kinds.iter().enumerate() {
        text += &content(u32::try_from(index).unwrap() + 1, name, "placeholder = true\n");
    }
    let tables = parse(&Sources { content: &text, ..minimal() }).unwrap();
    for (index, (_, kind)) in kinds.iter().enumerate() {
        let entry = tables.placeholder(*kind).unwrap();
        assert_eq!(entry.id, u32::try_from(index).unwrap() + 1);
    }
}

#[test]
fn a_bound_key_without_the_prefix_is_refused() {
    let sources = Sources {
        bounds: "[bounds]\nMAX_GIL = 1\nGIL = 2\n",
        ..minimal()
    };
    assert_eq!(rule_of(&sources), ("bounds.toml", "GIL".to_string(), "key without the MAX_ prefix"));
    let sources = Sources {
        bounds: "[bounds]\nmax_gil = 1\n",
        ..minimal()
    };
    assert_eq!(rule_of(&sources).1, "max_gil");
}

#[test]
fn a_negative_bound_is_refused() {
    let sources = Sources {
        bounds: "[bounds]\nMAX_GIL = -1\n",
        ..minimal()
    };
    assert_eq!(rule_of(&sources), ("bounds.toml", "MAX_GIL".to_string(), "negative bound"));
    let zero = Sources {
        bounds: "[bounds]\nMAX_GIL = 0\n",
        ..minimal()
    };
    assert_eq!(parse(&zero).unwrap().bound("MAX_GIL"), Some(0));
}

#[test]
fn the_message_names_file_row_and_rule() {
    let sources = Sources {
        bounds: "[bounds]\nMAX_GIL = -1\n",
        ..minimal()
    };
    let message = parse(&sources).unwrap_err().to_string();
    assert_eq!(message, "bounds.toml: MAX_GIL breaks a rule: negative bound");
}
