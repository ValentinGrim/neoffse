//! The restore target: the folder a manifest names and the open-file rule.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{T0, listing, make_backup, rewrite_manifest, slot_bytes, slot_file};
use neoffse::core::Game;
use neoffse::io::backup::Backup;
use neoffse::io::dirs::DataDir;
use neoffse_app::restore::{replaces_open_file, same_folder, target_of};
use tempfile::TempDir;

fn one_backup(source: &Path) -> (TempDir, DataDir, Backup) {
    let name = slot_file(1);
    fs::write(source.join(name), slot_bytes(1)).unwrap();
    let data_dir = tempfile::tempdir().unwrap();
    let data = DataDir::at(data_dir.path());
    make_backup(&data, "ff1-pr", source, &[name], "before-save", T0, 0);
    let backup = listing(&data, Game::Ff1Pr).remove(0);
    (data_dir, data, backup)
}

#[test]
fn target_of_reads_the_source_dir() {
    let source = tempfile::tempdir().unwrap();
    let (_keep, _data, backup) = one_backup(source.path());
    assert_eq!(target_of(&backup.manifest), Some(source.path().to_path_buf()));
}

#[test]
fn an_empty_source_dir_gives_none() {
    let source = tempfile::tempdir().unwrap();
    let (_keep, data, backup) = one_backup(source.path());
    rewrite_manifest(&backup, "sourceDir", "");
    let backup = listing(&data, Game::Ff1Pr).remove(0);
    assert_eq!(target_of(&backup.manifest), None);
}

#[test]
fn a_relative_source_dir_gives_none() {
    let source = tempfile::tempdir().unwrap();
    let (_keep, data, backup) = one_backup(source.path());
    rewrite_manifest(&backup, "sourceDir", "saves/slot");
    let backup = listing(&data, Game::Ff1Pr).remove(0);
    assert_eq!(target_of(&backup.manifest), None);
}

#[test]
fn same_folder_compares_paths() {
    let folder = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let path = folder.path();
    assert!(same_folder(path, path));
    assert!(same_folder(path, &path.join("")), "a trailing separator");
    assert!(same_folder(path, &path.join(".")), "a dot component");
    assert!(!same_folder(path, other.path()));

    let missing = path.join("missing");
    assert!(same_folder(&missing, &missing), "equal missing paths use the plain comparison");
    assert!(!same_folder(&missing, &path.join("other-missing")));
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_to_the_folder_is_the_same_folder() {
    let folder = tempfile::tempdir().unwrap();
    let links = tempfile::tempdir().unwrap();
    let link = links.path().join("link");
    std::os::unix::fs::symlink(folder.path(), &link).unwrap();
    assert!(same_folder(folder.path(), &link));
}

#[test]
fn replaces_open_file_needs_the_name_and_the_folder() {
    let folder = tempfile::tempdir().unwrap();
    let other: PathBuf = tempfile::tempdir().unwrap().path().to_path_buf();
    let name = slot_file(1);
    let single = [name.to_owned()];
    let several = [slot_file(2).to_owned(), name.to_owned(), "notes.txt".to_owned()];

    assert!(replaces_open_file(folder.path(), name, folder.path(), &single));
    assert!(replaces_open_file(folder.path(), name, folder.path(), &several));
    assert!(
        !replaces_open_file(folder.path(), slot_file(3), folder.path(), &single),
        "another name, same folder"
    );
    assert!(!replaces_open_file(folder.path(), name, &other, &single), "same name, another folder");
    assert!(
        !replaces_open_file(folder.path(), &name.to_uppercase(), folder.path(), &single),
        "the match is exact"
    );
}
