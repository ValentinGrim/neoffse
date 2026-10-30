//! `picker_entries` and `entry_name` over the FF I tables.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::core::{Game, TableKind};
use neoffse::data::{GameTables, Language, Sources, tables_for};
use neoffse_app::PickerEntry;
use neoffse_app::editor::{entry_name, picker_entries};

fn tables() -> &'static GameTables {
    tables_for(Game::Ff1Pr).expect("the FF I tables are embedded")
}

fn ids(entries: &[PickerEntry]) -> Vec<i32> {
    entries.iter().map(|entry| entry.id).collect()
}

#[test]
fn the_job_table_lists_every_entry_by_id() {
    let table = tables().table(TableKind::Job).expect("FF I has jobs");
    let entries = picker_entries(tables(), &[TableKind::Job], Language::En, "", false);
    assert_eq!(entries.len(), table.len());
    let found = ids(&entries);
    assert!(found.windows(2).all(|pair| pair[0] < pair[1]), "ids ascend");
    for (entry, row) in entries.iter().zip(table) {
        assert_eq!(i64::from(entry.id), i64::from(row.id));
        assert_eq!(Some(entry.name.as_str()), row.name_en.as_deref());
    }
}

#[test]
fn the_language_picks_the_name() {
    let table = tables().table(TableKind::Item).expect("FF I has items");
    let differing = table.iter().find(|entry| entry.name_en != entry.name_fr).expect("one item has two names");
    let id = i32::try_from(differing.id).unwrap();
    let find = |language| {
        picker_entries(tables(), &[TableKind::Item], language, "", false)
            .into_iter()
            .find(|entry| entry.id == id)
            .expect("the entry is listed")
    };
    let english = find(Language::En);
    let french = find(Language::Fr);
    assert_eq!(Some(english.name.as_str()), differing.name_en.as_deref());
    assert_eq!(Some(french.name.as_str()), differing.name_fr.as_deref());
    assert_ne!(english.name, french.name);
}

#[test]
fn a_query_filters_by_name_and_id() {
    let table = tables().table(TableKind::Job).expect("FF I has jobs");
    let named = table
        .iter()
        .find(|entry| entry.name_en.as_deref().is_some_and(|name| name.chars().count() >= 4))
        .expect("a job with a name");
    let id = i32::try_from(named.id).unwrap();
    let inner: String = named.name_en.as_deref().unwrap().chars().skip(1).take(3).collect();

    let expected: Vec<i32> = table
        .iter()
        .filter(|entry| entry.name_en.as_deref().is_some_and(|name| name.to_lowercase().contains(&inner.to_lowercase())))
        .map(|entry| i32::try_from(entry.id).unwrap())
        .collect();
    assert!(expected.contains(&id));
    for query in [inner.to_uppercase(), inner.to_lowercase()] {
        let entries = picker_entries(tables(), &[TableKind::Job], Language::En, &query, false);
        assert_eq!(ids(&entries), expected, "query {query:?}");
    }

    let by_id = picker_entries(tables(), &[TableKind::Job], Language::En, &named.id.to_string(), false);
    assert!(ids(&by_id).contains(&id), "a spelled id keeps its entry");
    assert!(by_id.iter().all(|entry| entry.id == id || entry.name.contains(&named.id.to_string())));

    assert_eq!(picker_entries(tables(), &[TableKind::Job], Language::En, "zzzzqq", false).len(), 0);
    let all = picker_entries(tables(), &[TableKind::Job], Language::En, "", false);
    assert_eq!(all.len(), table.len());
}

#[test]
fn a_query_ignores_surrounding_spaces() {
    let table = tables().table(TableKind::Job).expect("FF I has jobs");
    let named = table
        .iter()
        .find(|entry| entry.name_en.as_deref().is_some_and(|name| !name.is_empty()))
        .expect("a job with a name");
    let name = named.name_en.as_deref().unwrap();
    let id = i32::try_from(named.id).unwrap();
    let exact = picker_entries(tables(), &[TableKind::Job], Language::En, name, false);
    assert!(ids(&exact).contains(&id));
    for query in [format!("{name} "), format!("  {}\t", name.to_lowercase()), format!(" {} ", named.id)] {
        let entries = picker_entries(tables(), &[TableKind::Job], Language::En, &query, false);
        assert!(ids(&entries).contains(&id), "query {query:?} keeps the entry");
    }
    let padded = format!(" {name} ");
    assert_eq!(picker_entries(tables(), &[TableKind::Job], Language::En, &padded, false), exact);
    for blank in ["", " ", "   \t"] {
        let entries = picker_entries(tables(), &[TableKind::Job], Language::En, blank, false);
        assert_eq!(entries.len(), table.len(), "query {blank:?} lists every entry");
    }
}

#[test]
fn placeholders_drop_on_request() {
    let placeholder = tables().placeholder(TableKind::Weapon).expect("Weapon has a placeholder");
    let id = i32::try_from(placeholder.id).unwrap();
    let kept = picker_entries(tables(), &[TableKind::Weapon], Language::En, "", false);
    let dropped = picker_entries(tables(), &[TableKind::Weapon], Language::En, "", true);
    assert!(ids(&kept).contains(&id));
    assert!(!ids(&dropped).contains(&id));
    assert_eq!(dropped.len() + 1, kept.len());
}

#[test]
fn kinds_merge_sorted_by_id_and_skip_missing_tables() {
    let kinds = [TableKind::Item, TableKind::Weapon, TableKind::Map];
    let entries = picker_entries(tables(), &kinds, Language::En, "", false);
    let found = ids(&entries);
    assert!(found.windows(2).all(|pair| pair[0] <= pair[1]), "ids ascend");
    let items = tables().table(TableKind::Item).unwrap();
    let weapons = tables().table(TableKind::Weapon).unwrap();
    assert_eq!(entries.len(), items.len() + weapons.len());
    for entry in items.iter().chain(weapons) {
        assert!(found.contains(&i32::try_from(entry.id).unwrap()));
    }
    assert_eq!(picker_entries(tables(), &[TableKind::Map], Language::En, "", false).len(), 0);
}

#[test]
fn a_kind_listed_twice_counts_once() {
    let once = picker_entries(tables(), &[TableKind::Job], Language::En, "", false);
    let twice = picker_entries(tables(), &[TableKind::Job, TableKind::Job], Language::En, "", false);
    assert_eq!(once, twice);
}

#[test]
fn entry_name_finds_or_says_none() {
    let job = tables().table(TableKind::Job).unwrap().first().expect("a job");
    let id = i64::from(job.id);
    assert_eq!(entry_name(tables(), &[TableKind::Job], Language::En, id).as_deref(), job.name_en.as_deref());
    assert_eq!(entry_name(tables(), &[TableKind::Job], Language::Fr, id).as_deref(), job.name_fr.as_deref());
    assert_eq!(entry_name(tables(), &[TableKind::Job], Language::En, 13), None, "job 13 names no entry");
    assert_eq!(entry_name(tables(), &[TableKind::Job], Language::En, -1), None);
    assert_eq!(entry_name(tables(), &[TableKind::Job], Language::En, i64::MAX), None);
    assert_eq!(entry_name(tables(), &[TableKind::Map], Language::En, id), None);

    let first_hit = entry_name(tables(), &[TableKind::Map, TableKind::Job], Language::En, id);
    assert_eq!(first_hit.as_deref(), job.name_en.as_deref());
}

/// Tables whose only content is job 3, with no name in either language.
fn nameless_job_tables() -> GameTables {
    let sources = Sources {
        content: "content = []\n",
        abilities: "ability = []\n",
        jobs: "[[job]]\nid = 3\n",
        commands: "command = []\n",
        monsters: None,
        bounds: "[bounds]\n",
        growth: None,
    };
    GameTables::parse(Game::Ff1Pr, &sources).expect("synthetic tables parse")
}

#[test]
fn a_nameless_entry_is_listed_with_an_empty_name_and_found_by_id() {
    let tables = nameless_job_tables();
    let kinds = [TableKind::Job];
    let all = picker_entries(&tables, &kinds, Language::En, "", false);
    assert_eq!(all.len(), 1);
    assert_eq!((all[0].id, all[0].name.as_str()), (3, ""));
    assert_eq!(ids(&picker_entries(&tables, &kinds, Language::Fr, "3", false)), [3]);
    assert_eq!(picker_entries(&tables, &kinds, Language::En, "x", false).len(), 0);
}

#[test]
fn entry_name_gives_an_empty_text_for_a_nameless_entry() {
    let tables = nameless_job_tables();
    let kinds = [TableKind::Job];
    assert_eq!(entry_name(&tables, &kinds, Language::En, 3).as_deref(), Some(""));
    assert_eq!(entry_name(&tables, &kinds, Language::Fr, 3).as_deref(), Some(""));
    assert_eq!(entry_name(&tables, &kinds, Language::En, 4), None, "a missing id stays None");
}
