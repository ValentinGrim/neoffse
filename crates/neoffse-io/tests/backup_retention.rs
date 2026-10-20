//! Retention: twenty readable backups per game, the backup just made stays,
//! and what the crate cannot read is never deleted.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

use common::Step;
use common::backup::{BESTIARY, SLOT, Snoop, T0, World, at, game_dir, make, manifest_text, plant, request, tree, world};
use neoffse_io::atomic::RealFileOps;
use neoffse_io::backup::{create_with, list};
use neoffse_io::error::{IoError, Operation};

/// The folders `make` created, oldest first, for `count` instants from `T0`.
fn make_many(w: &World, game: &str, count: i64) -> Vec<PathBuf> {
    (0..count).map(|n| make(w, game, T0 + n).dir).collect()
}

fn listed_dirs(w: &World, game: &str) -> Vec<PathBuf> {
    list(&w.data, game).unwrap().backups.into_iter().map(|backup| backup.dir).collect()
}

#[test]
fn exactly_twenty_backups_are_kept() {
    let w = world("exactly_twenty_backups_are_kept");
    let dirs = make_many(&w, "ff1-pr", 20);
    assert!(dirs.iter().all(|dir| dir.is_dir()));
    assert_eq!(listed_dirs(&w, "ff1-pr").len(), 20);
}

#[test]
fn the_twenty_first_backup_removes_the_oldest() {
    let w = world("the_twenty_first_backup_removes_the_oldest");
    let dirs = make_many(&w, "ff1-pr", 21);
    let listed = listed_dirs(&w, "ff1-pr");
    assert_eq!(listed.len(), 20);
    assert!(!dirs[0].exists());
    assert!(dirs[1..].iter().all(|dir| dir.is_dir()));
    assert_eq!(listed[0], dirs[20]);
}

#[test]
fn retention_keeps_the_twenty_newest_of_many() {
    let w = world("retention_keeps_the_twenty_newest_of_many");
    let dirs = make_many(&w, "ff1-pr", 25);
    assert!(dirs[..5].iter().all(|dir| !dir.exists()));
    assert!(dirs[5..].iter().all(|dir| dir.is_dir()));
    let mut expected = dirs[5..].to_vec();
    expected.reverse();
    assert_eq!(listed_dirs(&w, "ff1-pr"), expected);
}

#[test]
fn retention_is_per_game() {
    let w = world("retention_is_per_game");
    make_many(&w, "ff1-pr", 20);
    make_many(&w, "ff2-pr", 3);
    make(&w, "ff2-pr", T0 + 100);
    assert_eq!(listed_dirs(&w, "ff1-pr").len(), 20);
    assert_eq!(listed_dirs(&w, "ff2-pr").len(), 4);
}

#[test]
fn a_backup_made_with_an_older_clock_survives() {
    let w = world("a_backup_made_with_an_older_clock_survives");
    let dirs: Vec<PathBuf> = (0..20).map(|n| make(&w, "ff1-pr", T0 + 100 + n).dir).collect();
    let files = [SLOT, BESTIARY];
    let made = create_with(&RealFileOps, &w.data, &request(&w, "ff1-pr", &files), at(T0)).unwrap();

    assert!(made.dir.is_dir());
    let listed = listed_dirs(&w, "ff1-pr");
    assert_eq!(listed.len(), 20);
    assert!(listed.contains(&made.dir));
    assert!(!dirs[0].exists());
    assert!(dirs[1..].iter().all(|dir| dir.is_dir()));
}

#[test]
fn unreadable_folders_are_neither_counted_nor_deleted() {
    let w = world("unreadable_folders_are_neither_counted_nor_deleted");
    let dirs = make_many(&w, "ff1-pr", 20);
    let future = manifest_text("ff1-pr", 2, &[SLOT]);
    let planted = [
        plant(&w, "ff1-pr", "20200101-000000", None),
        plant(&w, "ff1-pr", "20300101-000000", Some(&future)),
        plant(&w, "ff1-pr", "notes", None),
    ];
    let before: Vec<_> = planted.iter().map(|dir| tree(dir)).collect();

    let newest = make(&w, "ff1-pr", T0 + 20);
    let listing = list(&w.data, "ff1-pr").unwrap();
    assert_eq!(listing.backups.len(), 20);
    assert_eq!(listing.backups[0].dir, newest.dir);
    assert_eq!(listing.unreadable.len(), 2);
    assert!(!dirs[0].exists());
    assert!(dirs[1..].iter().all(|dir| dir.is_dir()));
    let after: Vec<_> = planted.iter().map(|dir| tree(dir)).collect();
    assert_eq!(after, before);
}

#[test]
fn a_failed_backup_prunes_nothing() {
    let w = world("a_failed_backup_prunes_nothing");
    make_many(&w, "ff1-pr", 20);
    // A 21st readable backup, older than all: the next success would delete it.
    plant(&w, "ff1-pr", "20200101-000000", Some(&manifest_text("ff1-pr", 1, &[SLOT])));
    let before = tree(&game_dir(&w, "ff1-pr"));

    let (snoop, _log) = Snoop::new(&[Step::Rename], false);
    let files = [SLOT, BESTIARY];
    let result = create_with(&snoop, &w.data, &request(&w, "ff1-pr", &files), at(T0 + 50));
    assert!(result.is_err());
    assert_eq!(tree(&game_dir(&w, "ff1-pr")), before);
}

#[test]
fn a_burst_in_one_second_keeps_the_newest_backups_and_never_reuses_a_name() {
    const STAMP: &str = "20261018-201403";
    let w = world("a_burst_in_one_second_keeps_the_newest_backups_and_never_reuses_a_name");
    for n in 0..100 {
        fs::write(w.source.join(SLOT), format!("gen-{n}")).unwrap();
        make(&w, "ff1-pr", T0);
    }

    let game = game_dir(&w, "ff1-pr");
    let listed = listed_dirs(&w, "ff1-pr");
    let expected: Vec<PathBuf> = (80..100).rev().map(|n| game.join(format!("{STAMP}-{n}"))).collect();
    assert_eq!(listed, expected);
    for (dir, n) in listed.iter().zip((80..100).rev()) {
        assert_eq!(fs::read(dir.join(SLOT)).unwrap(), format!("gen-{n}").into_bytes());
    }

    let before = tree(&game);
    let files = [SLOT, BESTIARY];
    let error = create_with(&RealFileOps, &w.data, &request(&w, "ff1-pr", &files), at(T0)).unwrap_err();
    let IoError::Io { operation, path, source } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(*operation, Operation::CreateDir);
    assert_eq!(*path, game.join(format!("{STAMP}-99")));
    assert_eq!(source.kind(), ErrorKind::AlreadyExists);
    assert_eq!(tree(&game), before);
}

#[test]
fn the_highest_existing_suffix_wins_over_a_free_lower_name() {
    let w = world("the_highest_existing_suffix_wins_over_a_free_lower_name");
    let game = game_dir(&w, "ff1-pr");
    fs::create_dir_all(game.join("20261018-201403-05")).unwrap();

    let made = make(&w, "ff1-pr", T0);
    assert_eq!(made.dir, game.join("20261018-201403-06"));
}

#[cfg(unix)]
#[test]
fn retention_failure_does_not_fail_the_backup() {
    use std::os::unix::fs::PermissionsExt as _;

    fn set_mode(path: &std::path::Path, mode: u32) {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    let w = world("retention_failure_does_not_fail_the_backup");
    let scratch = w.root.join("scratch");
    fs::create_dir_all(scratch.join("locked/inner")).unwrap();
    set_mode(&scratch.join("locked"), 0o555);
    let obstacle_holds = fs::remove_dir_all(&scratch).is_err();
    set_mode(&scratch.join("locked"), 0o755);
    if !obstacle_holds {
        return;
    }

    let dirs = make_many(&w, "ff1-pr", 20);
    let locked = dirs[0].join("locked");
    fs::create_dir_all(locked.join("inner")).unwrap();
    set_mode(&locked, 0o555);
    let files = [SLOT, BESTIARY];
    let result = create_with(&RealFileOps, &w.data, &request(&w, "ff1-pr", &files), at(T0 + 20));
    set_mode(&locked, 0o755);

    let made = result.unwrap();
    assert!(made.dir.is_dir());
    assert!(dirs[0].is_dir());
}
