//! Rows without a name (`docs/formats/pixel-remaster.md#rows-without-a-name`):
//! both fields absent is valid, one field alone is a rule error.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use common::{ff1, minimal, rule};
use neoffse_core::{Game, TableKind};
use neoffse_data::{DataError, GameTables, Language, Sources};

fn parse(sources: &Sources<'_>) -> Result<GameTables, DataError> {
    GameTables::parse(Game::Ff1Pr, sources)
}

/// One row with the given name lines, for each of the five files, as
/// `(file, table kind, sources)`.
fn cases(names: &str) -> Vec<(&'static str, TableKind, String)> {
    let row = |key: &str, extra: &str| format!("[[{key}]]\nid = 3\n{extra}{names}");
    vec![
        ("content.toml", TableKind::Item, row("content", "kind = \"item\"\n")),
        (
            "abilities.toml",
            TableKind::Ability,
            row("ability", "content_id = 9\nkind = \"command\"\nlevel = 0\n"),
        ),
        ("jobs.toml", TableKind::Job, row("job", "")),
        ("commands.toml", TableKind::Command, row("command", "")),
        ("monsters.toml", TableKind::Monster, row("monster", "")),
    ]
}

fn sources_with<'a>(file: &str, text: &'a str) -> Sources<'a> {
    let base = minimal();
    match file {
        "content.toml" => Sources { content: text, ..base },
        "abilities.toml" => Sources { abilities: text, ..base },
        "jobs.toml" => Sources { jobs: text, ..base },
        "commands.toml" => Sources { commands: text, ..base },
        _ => Sources { monsters: Some(text), ..base },
    }
}

#[test]
fn a_content_ability_job_command_and_monster_row_may_lack_names() {
    for (file, kind, text) in cases("") {
        let tables = parse(&sources_with(file, &text)).unwrap();
        let entry = tables.entry(kind, 3).unwrap();
        assert_eq!((entry.id, entry.kind), (3, kind), "{file}");
        assert_eq!(entry.name(Language::En), None, "{file}");
        assert_eq!(entry.name(Language::Fr), None, "{file}");
        assert_eq!((entry.name_en.as_deref(), entry.name_fr.as_deref()), (None, None), "{file}");
    }
}

#[test]
fn a_row_with_one_name_is_refused() {
    for line in ["name_en = \"A\"\n", "name_fr = \"B\"\n"] {
        for (file, _, text) in cases(line) {
            assert_eq!(
                rule(parse(&sources_with(file, &text))),
                (file, "id 3".to_string(), "name in one language only"),
                "{file}: {line}"
            );
        }
    }
}

#[test]
fn an_empty_name_is_still_a_name() {
    for (file, kind, text) in cases("name_en = \"\"\nname_fr = \"\"\n") {
        let tables = parse(&sources_with(file, &text)).unwrap();
        let entry = tables.entry(kind, 3).unwrap();
        assert_eq!(entry.name(Language::En), Some(""), "{file}");
        assert_eq!(entry.name(Language::Fr), Some(""), "{file}");
    }
}

#[test]
fn a_named_row_still_reads_both_languages() {
    let potion = ff1().entry(TableKind::Item, 2).unwrap();
    assert_eq!(potion.name(Language::En), Some("Potion"));
    assert_eq!(potion.name(Language::Fr), Some("Potion"));
    let ether = ff1().entry(TableKind::Item, 5).unwrap();
    assert_eq!((ether.name(Language::En), ether.name(Language::Fr)), (Some("Ether"), Some("Éther")));
}

#[test]
fn the_rule_reads_as_a_sentence() {
    let sources = Sources {
        jobs: "[[job]]\nid = 3\nname_fr = \"B\"\n",
        ..minimal()
    };
    assert_eq!(
        parse(&sources).unwrap_err().to_string(),
        "jobs.toml: id 3 breaks a rule: name in one language only"
    );
}
