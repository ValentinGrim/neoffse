//! Parse failures: the variant, the file and the id of each refusal.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::error::Error;

use common::minimal;
use neoffse_core::Game;
use neoffse_data::{DataError, GameTables, Sources};

fn parse(sources: &Sources<'_>) -> Result<GameTables, DataError> {
    GameTables::parse(Game::Ff1Pr, sources)
}

fn toml_file(result: Result<GameTables, DataError>) -> &'static str {
    match result.unwrap_err() {
        DataError::Toml { file, .. } => file,
        other => panic!("expected a Toml error, got {other}"),
    }
}

fn id_order(result: Result<GameTables, DataError>) -> (&'static str, u32) {
    match result.unwrap_err() {
        DataError::IdOrder { file, id } => (file, id),
        other => panic!("expected an IdOrder error, got {other}"),
    }
}

fn rows(key: &str, ids: &[u32]) -> String {
    let rows: Vec<String> = ids
        .iter()
        .map(|id| format!("[[{key}]]\nid = {id}\nname_en = \"A\"\nname_fr = \"B\"\n\n"))
        .collect();
    rows.concat()
}

#[test]
fn the_minimal_sources_parse() {
    assert!(parse(&minimal()).is_ok());
}

#[test]
fn malformed_toml_names_the_file() {
    let sources = Sources {
        content: "[[content\nid = 1",
        ..minimal()
    };
    let error = parse(&sources).unwrap_err();
    assert!(matches!(error, DataError::Toml { file: "content.toml", .. }));
    assert!(error.source().is_some());
    let sources = Sources {
        bounds: "[bounds\nMAX_GIL = 1",
        ..minimal()
    };
    assert_eq!(toml_file(parse(&sources)), "bounds.toml");
}

#[test]
fn an_unknown_field_is_refused() {
    let content = format!("{}sort_order = 3\n", common::CONTENT);
    let sources = Sources {
        content: &content,
        ..minimal()
    };
    assert_eq!(toml_file(parse(&sources)), "content.toml");
}

#[test]
fn a_missing_top_level_key_is_refused() {
    let sources = Sources {
        jobs: "[[jb]]\nid = 1\nname_en = \"A\"\nname_fr = \"B\"\n",
        ..minimal()
    };
    assert_eq!(toml_file(parse(&sources)), "jobs.toml");
}

#[test]
fn a_missing_name_is_refused() {
    let sources = Sources {
        content: "[[content]]\nid = 1\nkind = \"item\"\nname_en = \"A\"\n",
        ..minimal()
    };
    assert_eq!(toml_file(parse(&sources)), "content.toml");
}

#[test]
fn ids_outside_u32_are_toml_errors() {
    for id in ["-1", "4294967296"] {
        let content = format!("[[content]]\nid = {id}\nkind = \"item\"\nname_en = \"A\"\nname_fr = \"B\"\n");
        let sources = Sources {
            content: &content,
            ..minimal()
        };
        assert_eq!(toml_file(parse(&sources)), "content.toml", "id {id}");
    }
}

#[test]
fn a_repeated_key_is_refused() {
    let sources = Sources {
        bounds: "[bounds]\nMAX_GIL = 1\nMAX_GIL = 2\n",
        ..minimal()
    };
    assert_eq!(toml_file(parse(&sources)), "bounds.toml");
}

#[test]
fn an_unknown_content_kind_is_refused() {
    let content = "[[content]]\nid = 7\nkind = \"accessory\"\nname_en = \"A\"\nname_fr = \"B\"\n";
    let sources = Sources { content, ..minimal() };
    match parse(&sources).unwrap_err() {
        DataError::UnknownKind { file, id, kind } => {
            assert_eq!((file, id, kind.as_str()), ("content.toml", 7, "accessory"));
        }
        other => panic!("expected UnknownKind, got {other}"),
    }
}

#[test]
fn an_unknown_ability_kind_is_refused() {
    let abilities = "[[ability]]\nid = 1\ncontent_id = 209\nkind = \"spell\"\nlevel = 0\nname_en = \"A\"\nname_fr = \"B\"\n";
    let sources = Sources { abilities, ..minimal() };
    assert!(matches!(
        parse(&sources).unwrap_err(),
        DataError::UnknownKind {
            file: "abilities.toml",
            id: 1,
            ..
        }
    ));
}

fn content_row(id: u32) -> String {
    format!("[[content]]\nid = {id}\nkind = \"item\"\nname_en = \"A\"\nname_fr = \"B\"\n\n")
}

fn ability_row(id: u32) -> String {
    format!("[[ability]]\nid = {id}\ncontent_id = 300\nkind = \"monster\"\nlevel = 0\nname_en = \"A\"\nname_fr = \"B\"\n\n")
}

#[test]
fn id_zero_is_refused() {
    let content = content_row(0);
    let abilities = ability_row(0);
    let monsters = rows("monster", &[0]);
    assert_eq!(
        id_order(parse(&Sources {
            content: &content,
            ..minimal()
        })),
        ("content.toml", 0)
    );
    assert_eq!(
        id_order(parse(&Sources {
            abilities: &abilities,
            ..minimal()
        })),
        ("abilities.toml", 0)
    );
    assert_eq!(
        id_order(parse(&Sources {
            monsters: &monsters,
            ..minimal()
        })),
        ("monsters.toml", 0)
    );
}

#[test]
fn a_repeated_id_is_refused() {
    let content = content_row(4) + &content_row(4);
    let abilities = ability_row(4) + &ability_row(4);
    let monsters = rows("monster", &[3, 4, 4]);
    assert_eq!(
        id_order(parse(&Sources {
            content: &content,
            ..minimal()
        })),
        ("content.toml", 4)
    );
    assert_eq!(
        id_order(parse(&Sources {
            abilities: &abilities,
            ..minimal()
        })),
        ("abilities.toml", 4)
    );
    assert_eq!(
        id_order(parse(&Sources {
            monsters: &monsters,
            ..minimal()
        })),
        ("monsters.toml", 4)
    );
}

#[test]
fn a_descending_id_is_refused() {
    let content = content_row(6) + &content_row(5);
    let abilities = ability_row(6) + &ability_row(5);
    let monsters = rows("monster", &[6, 5]);
    assert_eq!(
        id_order(parse(&Sources {
            content: &content,
            ..minimal()
        })),
        ("content.toml", 5)
    );
    assert_eq!(
        id_order(parse(&Sources {
            abilities: &abilities,
            ..minimal()
        })),
        ("abilities.toml", 5)
    );
    assert_eq!(
        id_order(parse(&Sources {
            monsters: &monsters,
            ..minimal()
        })),
        ("monsters.toml", 5)
    );
}

#[test]
fn content_ids_are_unique_across_kinds() {
    let content = format!("{}[[content]]\nid = 5\nkind = \"weapon\"\nname_en = \"A\"\nname_fr = \"B\"\n", content_row(5));
    let sources = Sources {
        content: &content,
        ..minimal()
    };
    assert_eq!(id_order(parse(&sources)), ("content.toml", 5));
}

#[test]
fn the_first_error_wins() {
    let sources = Sources {
        content: "not toml [",
        bounds: "also [ not",
        ..minimal()
    };
    assert_eq!(toml_file(parse(&sources)), "content.toml");
}
