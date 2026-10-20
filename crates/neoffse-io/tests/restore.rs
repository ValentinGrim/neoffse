//! Restore, success paths: the backup files come back, the current files are
//! backed up first, and retention spares the backup being restored.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::backup::{BESTIARY, EDITED_BESTIARY, EDITED_SLOT, SLOT, T0, World, at, edit, game_dir, make, request, restore_at, tree, world};
use neoffse_io::atomic::RealFileOps;
use neoffse_io::backup::{MAX_BACKUPS, create_with, list, restore_with};
use neoffse_io::dirs::DataDir;

fn read(w: &World, name: &str) -> Vec<u8> {
    fs::read(w.source.join(name)).unwrap()
}

#[test]
fn restore_puts_the_backup_files_back() {
    let w = world("restore-restore_puts_the_backup_files_back");
    let backup = make(&w, "ff1-pr", T0);
    edit(&w);
    let before = tree(&backup.dir);

    let restored = restore_at(&w, &backup, T0 + 60).unwrap();
    assert_eq!(read(&w, SLOT), b"slot-one");
    assert_eq!(read(&w, BESTIARY), b"bestiary-one");
    assert_eq!(restored.created, Vec::<String>::new());
    assert_eq!(tree(&backup.dir), before);
}

#[test]
fn restore_backs_up_the_current_files_first() {
    let w = world("restore-restore_backs_up_the_current_files_first");
    let backup = make(&w, "ff1-pr", T0);
    edit(&w);

    let safety = restore_at(&w, &backup, T0 + 60).unwrap().safety.unwrap();
    assert_eq!(safety.dir, game_dir(&w, "ff1-pr").join("20261018-201503"));
    assert_eq!(safety.manifest.game, "ff1-pr");
    assert_eq!(safety.manifest.reason, "before-restore");
    assert_eq!(safety.manifest.files, [SLOT, BESTIARY]);
    assert_eq!(safety.manifest.source_dir, w.source.to_string_lossy());
    assert_eq!(safety.manifest.version, 1);
    assert_eq!(fs::read(safety.dir.join(SLOT)).unwrap(), EDITED_SLOT);
    assert_eq!(fs::read(safety.dir.join(BESTIARY)).unwrap(), EDITED_BESTIARY);
}

#[test]
fn the_safety_backup_takes_the_save_type_of_the_restored_manifest() {
    let w = world("restore-the_safety_backup_takes_the_save_type_of_the_restored_manifest");
    let files = [SLOT, BESTIARY];
    let autosave = neoffse_io::backup::Request {
        save_type: "autosave",
        ..request(&w, "ff1-pr", &files)
    };
    let backup = create_with(&RealFileOps, &w.data, &autosave, at(T0)).unwrap();
    edit(&w);

    let safety = restore_at(&w, &backup, T0 + 60).unwrap().safety.unwrap();
    assert_eq!(safety.manifest.save_type, "autosave");
}

#[test]
fn a_restore_can_be_undone() {
    let w = world("restore-a_restore_can_be_undone");
    let backup = make(&w, "ff1-pr", T0);
    edit(&w);

    let safety = restore_at(&w, &backup, T0 + 60).unwrap().safety.unwrap();
    assert_eq!(read(&w, SLOT), b"slot-one");
    restore_at(&w, &safety, T0 + 120).unwrap();
    assert_eq!(read(&w, SLOT), EDITED_SLOT);
    assert_eq!(read(&w, BESTIARY), EDITED_BESTIARY);
}

#[test]
fn restore_touches_only_the_files_of_the_backup() {
    let w = world("restore-restore_touches_only_the_files_of_the_backup");
    let backup = create_with(&RealFileOps, &w.data, &request(&w, "ff1-pr", &[SLOT]), at(T0)).unwrap();
    fs::write(w.source.join("notes.txt"), b"my notes").unwrap();
    edit(&w);

    let safety = restore_at(&w, &backup, T0 + 60).unwrap().safety.unwrap();
    assert_eq!(read(&w, SLOT), b"slot-one");
    assert_eq!(read(&w, BESTIARY), EDITED_BESTIARY);
    assert_eq!(read(&w, "notes.txt"), b"my notes");
    assert_eq!(safety.manifest.files, [SLOT]);
}

#[test]
fn files_missing_in_the_target_are_created() {
    let w = world("restore-files_missing_in_the_target_are_created");
    let backup = make(&w, "ff1-pr", T0);
    edit(&w);
    fs::remove_file(w.source.join(BESTIARY)).unwrap();

    let restored = restore_at(&w, &backup, T0 + 60).unwrap();
    assert_eq!(restored.created, [BESTIARY]);
    assert_eq!(read(&w, BESTIARY), b"bestiary-one");
    assert_eq!(restored.safety.unwrap().manifest.files, [SLOT]);
}

#[test]
fn an_empty_target_gets_no_safety_backup() {
    let w = world("restore-an_empty_target_gets_no_safety_backup");
    let backup = make(&w, "ff1-pr", T0);
    fs::remove_file(w.source.join(SLOT)).unwrap();
    fs::remove_file(w.source.join(BESTIARY)).unwrap();
    let games_before = tree(&game_dir(&w, "ff1-pr"));

    let restored = restore_at(&w, &backup, T0 + 60).unwrap();
    assert_eq!(restored.safety, None);
    assert_eq!(restored.created, [SLOT, BESTIARY]);
    assert_eq!(tree(&game_dir(&w, "ff1-pr")), games_before);
    assert_eq!(read(&w, SLOT), b"slot-one");
    assert_eq!(read(&w, BESTIARY), b"bestiary-one");
}

#[test]
fn restoring_the_oldest_of_twenty_backups_keeps_it() {
    let w = world("restore-restoring_the_oldest_of_twenty_backups_keeps_it");
    for n in 0..20 {
        fs::write(w.source.join(SLOT), format!("gen-{n}")).unwrap();
        make(&w, "ff1-pr", T0 + n);
    }
    edit(&w);
    let oldest = list(&w.data, "ff1-pr").unwrap().backups.pop().unwrap();
    let before = tree(&oldest.dir);

    let safety = restore_at(&w, &oldest, T0 + 100).unwrap().safety.unwrap();
    assert_eq!(read(&w, SLOT), b"gen-0");
    assert!(oldest.dir.is_dir());
    assert_eq!(tree(&oldest.dir), before);
    let listed = list(&w.data, "ff1-pr").unwrap().backups;
    assert_eq!(listed.len(), 21);
    assert!(listed.iter().any(|backup| backup.dir == safety.dir));
    assert!(listed.iter().any(|backup| backup.dir == oldest.dir));

    make(&w, "ff1-pr", T0 + 200);
    let listed = list(&w.data, "ff1-pr").unwrap().backups;
    assert_eq!(listed.len(), 20);
    assert!(!oldest.dir.exists());
}

#[test]
fn restore_protects_its_backup_under_another_spelling_of_the_data_root() {
    let w = world("restore-restore_protects_its_backup_under_another_spelling_of_the_data_root");
    for n in 0..MAX_BACKUPS {
        fs::write(w.source.join(SLOT), format!("gen-{n}")).unwrap();
        make(&w, "ff1-pr", T0 + i64::try_from(n).unwrap());
    }
    edit(&w);
    let root = w.data.root().to_path_buf();
    fs::create_dir(root.join("sub")).unwrap();
    let spelled = DataDir::at(root.join("sub").join(".."));
    let oldest = list(&spelled, "ff1-pr").unwrap().backups.pop().unwrap();
    assert_ne!(oldest.dir.parent(), Some(game_dir(&w, "ff1-pr").as_path()));

    restore_with(&RealFileOps, &w.data, "ff1-pr", &oldest, &w.source, at(T0 + 100)).unwrap();

    assert_eq!(read(&w, SLOT), b"gen-0");
    assert!(oldest.dir.is_dir());
    assert_eq!(list(&w.data, "ff1-pr").unwrap().backups.len(), MAX_BACKUPS + 1);
}

#[test]
fn a_restore_uses_a_backup_taken_from_list() {
    let w = world("restore-a_restore_uses_a_backup_taken_from_list");
    make(&w, "ff1-pr", T0);
    edit(&w);
    let listed = list(&w.data, "ff1-pr").unwrap().backups.remove(0);

    restore_at(&w, &listed, T0 + 60).unwrap();
    assert_eq!(read(&w, SLOT), b"slot-one");
    assert_eq!(read(&w, BESTIARY), b"bestiary-one");
}

#[test]
fn an_undo_keeps_the_files_the_restore_created() {
    let w = world("restore-an_undo_keeps_the_files_the_restore_created");
    let backup = make(&w, "ff1-pr", T0);
    edit(&w);
    fs::remove_file(w.source.join(BESTIARY)).unwrap();

    let restored = restore_at(&w, &backup, T0 + 60).unwrap();
    assert_eq!(restored.created, [BESTIARY]);
    let safety = restored.safety.unwrap();
    assert_eq!(safety.manifest.files, [SLOT]);

    let undone = restore_at(&w, &safety, T0 + 120).unwrap();
    assert_eq!(undone.created, Vec::<String>::new());
    assert_eq!(read(&w, SLOT), EDITED_SLOT);
    assert_eq!(read(&w, BESTIARY), b"bestiary-one");
}

#[cfg(unix)]
#[test]
fn a_link_at_a_target_name_is_replaced_by_the_restored_file() {
    use std::os::unix::fs::symlink;

    let w = world("restore-a_link_at_a_target_name_is_replaced_by_the_restored_file");
    let backup = make(&w, "ff1-pr", T0);
    let outside = w.root.join("outside");
    fs::write(&outside, b"linked-content").unwrap();
    fs::remove_file(w.source.join(SLOT)).unwrap();
    symlink(&outside, w.source.join(SLOT)).unwrap();

    let restored = restore_at(&w, &backup, T0 + 60).unwrap();
    let entry = fs::symlink_metadata(w.source.join(SLOT)).unwrap();
    assert!(entry.is_file() && !entry.file_type().is_symlink());
    assert_eq!(read(&w, SLOT), b"slot-one");
    assert_eq!(fs::read(&outside).unwrap(), b"linked-content");
    assert_eq!(restored.created, Vec::<String>::new());
    let safety = restored.safety.unwrap();
    assert_eq!(fs::read(safety.dir.join(SLOT)).unwrap(), b"linked-content");
}

#[cfg(unix)]
#[test]
fn a_dangling_link_at_a_target_name_fails_the_safety_backup() {
    use std::os::unix::fs::symlink;

    use neoffse_io::error::{IoError, Operation};

    let w = world("restore-a_dangling_link_at_a_target_name_fails_the_safety_backup");
    let backup = make(&w, "ff1-pr", T0);
    fs::remove_file(w.source.join(SLOT)).unwrap();
    let nowhere = w.root.join("nowhere");
    symlink(&nowhere, w.source.join(SLOT)).unwrap();
    // `tree` follows links and cannot read a dangling one.
    let names = |w: &World| {
        let mut found: Vec<String> = fs::read_dir(&w.source)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        found
    };
    let before = (names(&w), tree(&game_dir(&w, "ff1-pr")));

    let error = restore_at(&w, &backup, T0 + 60).unwrap_err();
    let IoError::Io { operation, path, .. } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(*operation, Operation::Copy);
    assert_eq!(*path, w.source.join(SLOT));
    assert_eq!((names(&w), tree(&game_dir(&w, "ff1-pr"))), before);
    assert_eq!(fs::read_link(w.source.join(SLOT)).unwrap(), nowhere);
    assert_eq!(read(&w, BESTIARY), b"bestiary-one");
}
