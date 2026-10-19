//! Listing the backups of a game: order, validation and what is ignored.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::io::ErrorKind;

use common::backup::{BESTIARY, SLOT, T0, game_dir, make, manifest_text, plant, world};
use neoffse_io::backup::list;
use neoffse_io::error::{IoError, Operation};

fn folder_names(backups: &[neoffse_io::backup::Backup]) -> Vec<&str> {
    backups.iter().map(|backup| backup.dir.file_name().unwrap().to_str().unwrap()).collect()
}

#[test]
fn a_game_without_backups_lists_empty() {
    let w = world("a_game_without_backups_lists_empty");
    let listing = list(&w.data, "ff1-pr").unwrap();
    assert_eq!(listing.backups, []);
    assert!(listing.unreadable.is_empty());
    assert!(!w.data.root().exists());
}

#[test]
fn list_returns_newest_first() {
    let w = world("list_returns_newest_first");
    for unix in [T0 + 20, T0, T0 + 10, T0 + 10] {
        make(&w, "ff1-pr", unix);
    }
    let listing = list(&w.data, "ff1-pr").unwrap();
    let expected = ["20261018-201423", "20261018-201413-01", "20261018-201413", "20261018-201403"];
    assert_eq!(folder_names(&listing.backups), expected);
    assert!(listing.unreadable.is_empty());
}

#[test]
fn list_reads_back_what_create_wrote() {
    let w = world("list_reads_back_what_create_wrote");
    let made = [make(&w, "ff1-pr", T0), make(&w, "ff1-pr", T0 + 5), make(&w, "ff1-pr", T0 + 5)];
    let listing = list(&w.data, "ff1-pr").unwrap();
    assert_eq!(listing.backups.len(), 3);
    for (found, made) in listing.backups.iter().zip(made.iter().rev()) {
        assert_eq!(found.dir, made.dir);
        assert_eq!(found.manifest, made.manifest);
    }
}

#[test]
fn list_ignores_names_that_are_not_backup_stamps() {
    let w = world("list_ignores_names_that_are_not_backup_stamps");
    let valid = manifest_text("ff1-pr", 1, &[SLOT]);
    for name in [
        "notes",
        "20261018-221403-001",
        "2026101-221403",
        "20261018_221403",
        "20261018-22140a",
        "20261018-221403-1",
    ] {
        plant(&w, "ff1-pr", name, Some(&valid));
    }
    fs::write(game_dir(&w, "ff1-pr").join("20261018-221403"), b"a file").unwrap();
    let listing = list(&w.data, "ff1-pr").unwrap();
    assert_eq!(listing.backups, []);
    assert!(listing.unreadable.is_empty());
}

#[cfg(unix)]
#[test]
fn list_ignores_a_symbolic_link_named_like_a_stamp() {
    let w = world("list_ignores_a_symbolic_link_named_like_a_stamp");
    let target = plant(&w, "ff1-pr", "notes", Some(&manifest_text("ff1-pr", 1, &[SLOT])));
    std::os::unix::fs::symlink(&target, game_dir(&w, "ff1-pr").join("20261018-221403")).unwrap();
    let listing = list(&w.data, "ff1-pr").unwrap();
    assert_eq!(listing.backups, []);
    assert!(listing.unreadable.is_empty());
}

#[test]
fn list_reports_an_incomplete_folder() {
    let w = world("list_reports_an_incomplete_folder");
    let dir = plant(&w, "ff1-pr", "20261018-221403", None);
    let listing = list(&w.data, "ff1-pr").unwrap();
    assert_eq!(listing.backups, []);
    assert_eq!(listing.unreadable.len(), 1);
    let IoError::Io { operation, path, source } = &listing.unreadable[0] else {
        panic!("{:?}", listing.unreadable[0]);
    };
    assert_eq!(*operation, Operation::Read);
    assert_eq!(*path, dir.join("backup.json"));
    assert_eq!(source.kind(), ErrorKind::NotFound);
    assert!(dir.join(SLOT).is_file());
}

#[test]
fn list_reports_invalid_manifests() {
    let valid = manifest_text("ff1-pr", 1, &[SLOT]);
    let truncated = valid[..valid.len() / 2].to_owned();
    let reason = "\"reason\": \"before-save\"";
    let cases: Vec<(String, Option<&str>)> = vec![
        ("this is not json".to_owned(), None),
        (truncated, None),
        (format!("{valid}x"), Some("trailing characters")),
        (valid.replace(&format!(",\n  {reason}"), ""), Some("missing field")),
        (valid.replace("\"version\": 1", "\"version\": \"1\""), Some("invalid type")),
        (manifest_text("ff1-pr", 2, &[SLOT]), Some("unsupported manifest version 2")),
        (manifest_text("ff1-pr", 0, &[SLOT]), Some("unsupported manifest version 0")),
        (manifest_text("ff2-pr", 1, &[SLOT]), Some("belongs to game")),
        (manifest_text("ff1-pr", 1, &["../x"]), Some("not a plain file name")),
        (manifest_text("ff1-pr", 1, &["backup.json"]), Some("not a plain file name")),
        (manifest_text("ff1-pr", 1, &[]), Some("at least one file")),
        (manifest_text("ff1-pr", 1, &[SLOT, SLOT]), Some("appears twice")),
        (valid.replace(reason, "\"reason\": \"\""), Some("reason is empty")),
    ];
    for (n, (text, phrase)) in cases.iter().enumerate() {
        let w = world(&format!("list_reports_invalid_manifests-{n}"));
        let dir = plant(&w, "ff1-pr", "20261018-221403", Some(text));
        let listing = list(&w.data, "ff1-pr").unwrap();
        assert!(listing.backups.is_empty(), "case {n}");
        assert_eq!(listing.unreadable.len(), 1, "case {n}");
        let IoError::ManifestInvalid { path, source } = &listing.unreadable[0] else {
            panic!("case {n}: {:?}", listing.unreadable[0]);
        };
        assert_eq!(*path, dir.join("backup.json"), "case {n}");
        if let Some(phrase) = phrase {
            assert!(source.to_string().contains(phrase), "case {n}: {source}");
        }
    }

    let w = world("list_reports_invalid_manifests-extra");
    plant(
        &w,
        "ff1-pr",
        "20261018-221403",
        Some(&valid.replace("\"version\": 1,", "\"version\": 1,\n  \"extra\": 1,")),
    );
    let listing = list(&w.data, "ff1-pr").unwrap();
    assert_eq!(listing.backups.len(), 1);
    assert!(listing.unreadable.is_empty());
}

#[test]
fn list_orders_unreadable_folders_newest_first() {
    let w = world("list_orders_unreadable_folders_newest_first");
    plant(&w, "ff1-pr", "20261018-000001", None);
    plant(&w, "ff1-pr", "20261018-000003", Some(&manifest_text("ff1-pr", 2, &[SLOT])));
    plant(&w, "ff1-pr", "20261018-000002", Some(&manifest_text("ff1-pr", 1, &[SLOT, BESTIARY])));
    let listing = list(&w.data, "ff1-pr").unwrap();
    assert_eq!(folder_names(&listing.backups), ["20261018-000002"]);
    let paths: Vec<_> = listing
        .unreadable
        .iter()
        .map(|error| match error {
            IoError::Io { path, .. } | IoError::ManifestInvalid { path, .. } => path.parent().unwrap().file_name().unwrap().to_str().unwrap().to_owned(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(paths, ["20261018-000003", "20261018-000001"]);
}

#[test]
fn list_is_per_game() {
    let w = world("list_is_per_game");
    let one = make(&w, "ff1-pr", T0);
    make(&w, "ff2-pr", T0 + 1);
    make(&w, "ff2-pr", T0 + 2);
    let first = list(&w.data, "ff1-pr").unwrap();
    assert_eq!(first.backups.len(), 1);
    assert_eq!(first.backups[0].dir, one.dir);
    let second = list(&w.data, "ff2-pr").unwrap();
    assert_eq!(second.backups.len(), 2);
    assert!(second.backups.iter().all(|backup| backup.manifest.game == "ff2-pr"));
}

#[test]
fn list_rejects_an_invalid_game_id() {
    let w = world("list_rejects_an_invalid_game_id");
    let error = list(&w.data, "../x").unwrap_err();
    assert!(matches!(error, IoError::InvalidGameId { .. }), "{error:?}");
}

#[test]
fn list_fails_when_the_game_folder_is_a_file() {
    let w = world("list_fails_when_the_game_folder_is_a_file");
    let path = game_dir(&w, "ff1-pr");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"not a folder").unwrap();
    let error = list(&w.data, "ff1-pr").unwrap_err();
    let IoError::Io {
        operation,
        path: found,
        source,
    } = &error
    else {
        panic!("{error:?}");
    };
    assert_eq!(*operation, Operation::ListDir);
    assert_eq!(*found, path);
    assert_ne!(source.kind(), ErrorKind::NotFound);
}
