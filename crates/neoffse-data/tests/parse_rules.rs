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
    format!("[[ability]]\nid = {id}\ncontent_id = {content_id}\ntype_id = 1\nkind = \"white_magic\"\nlevel = {level}\nname_en = \"A\"\nname_fr = \"B\"\n")
}

/// An ability row of the given `type_id` and `kind` string, with no content id.
fn typed(id: u32, type_id: u32, kind: &str) -> String {
    format!("[[ability]]\nid = {id}\ntype_id = {type_id}\nkind = \"{kind}\"\nlevel = 0\nname_en = \"A\"\nname_fr = \"B\"\n\n")
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

#[test]
fn a_type_id_outside_the_kind_table_is_refused() {
    for type_id in [0, 39, 999] {
        let text = typed(1, type_id, "command");
        let sources = Sources { abilities: &text, ..minimal() };
        assert_eq!(
            rule_of(&sources),
            ("abilities.toml", "id 1".to_string(), "type_id outside the kind table"),
            "type_id {type_id}"
        );
    }
}

#[test]
fn a_kind_that_differs_from_its_type_id_is_refused() {
    for (type_id, kind) in [(1, "black_magic"), (3, "command"), (638, "command"), (4, "internal")] {
        let text = typed(1, type_id, kind);
        let sources = Sources { abilities: &text, ..minimal() };
        assert_eq!(
            rule_of(&sources),
            ("abilities.toml", "id 1".to_string(), "kind differs from type_id"),
            "type_id {type_id}"
        );
    }
}

#[test]
fn a_kind_that_follows_its_type_id_parses() {
    let text = typed(1, 13, "monster") + &typed(2, 36, "command") + &typed(3, 41, "internal");
    assert!(parse(&Sources { abilities: &text, ..minimal() }).is_ok());
}

#[test]
fn an_unknown_kind_string_comes_before_the_type_id_rules() {
    let text = typed(1, 999, "spell");
    let sources = Sources { abilities: &text, ..minimal() };
    assert!(matches!(
        parse(&sources).unwrap_err(),
        DataError::UnknownKind {
            file: "abilities.toml",
            id: 1,
            ..
        }
    ));
}

#[test]
fn a_repeated_content_id_is_refused() {
    let text = ability(1, 209, 1) + "\n" + &ability(2, 300, 1) + "\n" + &ability(3, 209, 1);
    let sources = Sources { abilities: &text, ..minimal() };
    assert_eq!(rule_of(&sources), ("abilities.toml", "id 3".to_string(), "content_id repeats"));
}

#[test]
fn rows_without_a_content_id_never_repeat() {
    let text = typed(1, 1, "white_magic") + &typed(2, 1, "white_magic") + &ability(3, 209, 1);
    let tables = parse(&Sources { abilities: &text, ..minimal() }).unwrap();
    assert_eq!(tables.ability_by_content(209).map(|entry| entry.id), Some(3));
}

#[test]
fn an_equipment_kind_without_a_placeholder_is_refused() {
    for kind in ["weapon", "shield", "armor", "helmet", "gloves"] {
        let text = content(1, "item", "") + &content(2, kind, "");
        let sources = Sources { content: &text, ..minimal() };
        assert_eq!(
            rule_of(&sources),
            ("content.toml", kind.to_string(), "equipment kind without a placeholder"),
            "{kind}"
        );
    }
}

#[test]
fn an_equipment_kind_with_no_row_needs_no_placeholder() {
    let text = content(1, "item", "") + &content(2, "key_item", "");
    assert!(parse(&Sources { content: &text, ..minimal() }).is_ok());
}

#[test]
fn a_present_table_with_an_empty_array_is_refused() {
    let sources = Sources {
        content: "content = []\n",
        ..minimal()
    };
    assert_eq!(rule_of(&sources), ("content.toml", "content".to_string(), "table has no row"));
    let sources = Sources {
        abilities: "ability = []\n",
        ..minimal()
    };
    assert_eq!(rule_of(&sources), ("abilities.toml", "ability".to_string(), "table has no row"));
    let sources = Sources {
        jobs: "job = []\n",
        ..minimal()
    };
    assert_eq!(rule_of(&sources), ("jobs.toml", "job".to_string(), "table has no row"));
    let sources = Sources {
        commands: "command = []\n",
        ..minimal()
    };
    assert_eq!(rule_of(&sources), ("commands.toml", "command".to_string(), "table has no row"));
    let sources = Sources {
        monsters: Some("monster = []\n"),
        ..minimal()
    };
    assert_eq!(rule_of(&sources), ("monsters.toml", "monster".to_string(), "table has no row"));
}

#[test]
fn an_absent_monsters_file_is_not_an_empty_table() {
    let tables = parse(&Sources { monsters: None, ..minimal() }).unwrap();
    assert!(tables.table(TableKind::Monster).is_none());
}
