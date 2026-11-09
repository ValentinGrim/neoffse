//! The content kinds `keyword`, `magic_stone` and `command_content`
//! (`docs/formats/pixel-remaster.md#content-types-5-to-9`).
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{id_order, minimal, rule};
use neoffse_core::{Game, TableKind};
use neoffse_data::{DataError, GameTables, Language, Sources};

const NEW_KINDS: [(&str, TableKind); 3] = [
    ("keyword", TableKind::Keyword),
    ("magic_stone", TableKind::MagicStone),
    ("command_content", TableKind::CommandContent),
];

fn parse(content: &str) -> Result<GameTables, DataError> {
    GameTables::parse(Game::Ff1Pr, &Sources { content, ..minimal() })
}

/// An item at id 1, then one row of each new kind at ids 2 to 4.
fn mixed() -> String {
    let item = "[[content]]\nid = 1\nkind = \"item\"\nname_en = \"Potion\"\nname_fr = \"Potion\"\n";
    let rows: Vec<String> = NEW_KINDS
        .iter()
        .zip(2..)
        .map(|((kind, _), id)| format!("[[content]]\nid = {id}\nkind = \"{kind}\"\nname_en = \"E{id}\"\nname_fr = \"F{id}\"\n"))
        .collect();
    format!("{item}\n{}", rows.join("\n"))
}

#[test]
fn the_three_new_kinds_parse() {
    let tables = parse(&mixed()).unwrap();
    for (index, (_, kind)) in NEW_KINDS.iter().enumerate() {
        let id = u32::try_from(index).unwrap() + 2;
        let entry = tables.entry(*kind, id).unwrap();
        assert_eq!((entry.id, entry.kind), (id, *kind));
        assert_eq!(entry.name(Language::En), Some(format!("E{id}").as_str()));
        assert_eq!(tables.table(*kind).unwrap().len(), 1, "{kind:?}");
        assert!(tables.entry(*kind, 1).is_none(), "{kind:?}: id 1 is an item");
    }
}

#[test]
fn a_new_kind_is_not_a_content_lookup_kind() {
    let tables = parse(&mixed()).unwrap();
    assert!(tables.content(1).is_some());
    for id in 2..=4 {
        assert!(tables.content(id).is_none(), "id {id}");
    }
}

#[test]
fn ids_rise_across_kinds() {
    let text = "[[content]]\nid = 5\nkind = \"item\"\nname_en = \"A\"\nname_fr = \"B\"\n\n\
                [[content]]\nid = 3\nkind = \"keyword\"\nname_en = \"A\"\nname_fr = \"B\"\n";
    assert_eq!(id_order(parse(text)), ("content.toml", 3));
}

#[test]
fn a_placeholder_on_a_new_kind_is_refused() {
    for (kind, _) in NEW_KINDS {
        let text = format!("[[content]]\nid = 2\nkind = \"{kind}\"\nplaceholder = true\nname_en = \"A\"\nname_fr = \"B\"\n");
        assert_eq!(
            rule(parse(&text)),
            ("content.toml", "id 2".to_string(), "placeholder on a non-equipment kind"),
            "{kind}"
        );
    }
}

#[test]
fn a_nameless_keyword_parses() {
    let tables = parse("[[content]]\nid = 7\nkind = \"keyword\"\n").unwrap();
    let entry = tables.entry(TableKind::Keyword, 7).unwrap();
    assert_eq!((entry.name(Language::En), entry.name(Language::Fr)), (None, None));
}

#[test]
fn an_unknown_content_kind_is_still_refused() {
    match parse("[[content]]\nid = 7\nkind = \"relic\"\nname_en = \"A\"\nname_fr = \"B\"\n").unwrap_err() {
        DataError::UnknownKind { file, id, kind } => assert_eq!((file, id, kind.as_str()), ("content.toml", 7, "relic")),
        other => panic!("expected UnknownKind, got {other}"),
    }
}
