//! Windows discovery, fed by hand with the Windows fields only so that it
//! runs on every system.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;
use std::path::PathBuf;

use common::discovery::{GAME_A, GAME_B, ID_1, ID_2, documents_save, paths, prefix, site, steam_dir, steam_root};
use neoffse_io::discovery::{Environment, Origin, discover};

fn windows(documents: Option<&PathBuf>, user_profile: Option<&PathBuf>, one_drive: Option<&PathBuf>) -> Environment {
    Environment {
        documents: documents.cloned(),
        user_profile: user_profile.cloned(),
        one_drive: one_drive.cloned(),
        ..Environment::default()
    }
}

#[test]
fn the_documents_known_folder_is_searched() {
    let site = site("the_documents_known_folder_is_searched");
    let documents = site.base.join("docs");
    let save = documents_save(&documents, &GAME_A, ID_1);

    let found = discover(&windows(Some(&documents), None, None), &[GAME_A]);

    assert_eq!(paths(&found), [save]);
    assert_eq!(found.folders[0].game, "test-a");
    assert_eq!(found.folders[0].steam_id, ID_1);
    assert_eq!(found.folders[0].origin, Origin::Documents { documents });
    assert!(found.skipped.is_empty());
}

#[test]
fn each_fallback_finds_saves_on_its_own() {
    let site = site("each_fallback_finds_saves_on_its_own");
    let profile = site.base.join("profile");
    let drive = site.base.join("drive");
    let places = [
        (profile.join("Documents"), windows(None, Some(&profile), None)),
        (profile.join("OneDrive/Documents"), windows(None, Some(&profile), None)),
        (drive.join("Documents"), windows(None, None, Some(&drive))),
    ];
    for (documents, environment) in places {
        let save = documents_save(&documents, &GAME_A, ID_1);

        let found = discover(&environment, &[GAME_A]);

        assert_eq!(paths(&found), [save]);
        assert_eq!(found.folders[0].origin, Origin::Documents { documents: documents.clone() });
        fs::remove_dir_all(documents.join("My Games")).unwrap();
    }
}

#[test]
fn the_same_folder_through_several_candidates_counts_once() {
    let site = site("the_same_folder_through_several_candidates_counts_once");
    let profile = site.base.join("profile");
    let drive = profile.join("OneDrive");
    let environment = windows(Some(&profile.join("Documents")), Some(&profile), Some(&drive));
    let local = documents_save(&profile.join("Documents"), &GAME_A, ID_1);

    assert_eq!(paths(&discover(&environment, &[GAME_A])), std::slice::from_ref(&local));

    let synced = documents_save(&drive.join("Documents"), &GAME_A, ID_2);
    assert_eq!(paths(&discover(&environment, &[GAME_A])), [local, synced]);
}

#[cfg(unix)]
#[test]
fn the_same_folder_through_a_link_counts_once() {
    let site = site("the_same_folder_through_a_link_counts_once");
    let profile = site.base.join("profile");
    let link = site.base.join("link");
    documents_save(&profile.join("Documents"), &GAME_A, ID_1);
    std::os::unix::fs::symlink(profile.join("Documents"), &link).unwrap();

    let found = discover(&windows(Some(&link), Some(&profile), None), &[GAME_A]);

    assert_eq!(found.folders.len(), 1);
    assert_eq!(found.folders[0].origin, Origin::Documents { documents: link.clone() });
    assert_eq!(found.folders[0].path, link.join("My Games/TEST GAME A PR/Steam").join(ID_1));
}

#[test]
fn two_distinct_documents_folders_both_report() {
    let site = site("two_distinct_documents_folders_both_report");
    let profile = site.base.join("profile");
    let synced = site.base.join("onedrive/Documents");
    let remote = documents_save(&synced, &GAME_A, ID_1);
    let local = documents_save(&profile.join("Documents"), &GAME_A, ID_2);

    let found = discover(&windows(Some(&synced), Some(&profile), None), &[GAME_A]);

    assert_eq!(paths(&found), [remote, local]);
}

#[test]
fn the_steam_id_and_game_rules_apply_there_too() {
    let site = site("the_steam_id_and_game_rules_apply_there_too");
    let documents = site.base.join("docs");
    let second = documents_save(&documents, &GAME_A, ID_2);
    let first = documents_save(&documents, &GAME_A, ID_1);
    let steam = steam_dir(&documents, &GAME_A);
    for name in ["7656000000000000", "765600000000000000", "76550000000000001", "7656000000000000a", "notes"] {
        fs::create_dir(steam.join(name)).unwrap();
    }
    fs::write(steam.join("76560000000000003"), b"a file").unwrap();
    documents_save(&documents, &GAME_B, ID_1);

    let found = discover(&windows(Some(&documents), None, None), &[GAME_A]);

    assert_eq!(paths(&found), [first, second]);
}

#[test]
fn a_missing_documents_folder_is_silent() {
    let site = site("a_missing_documents_folder_is_silent");
    let (docs, profile, drive) = (site.base.join("docs"), site.base.join("profile"), site.base.join("drive"));

    let found = discover(&windows(Some(&docs), Some(&profile), Some(&drive)), &[GAME_A]);

    assert!(found.folders.is_empty() && found.skipped.is_empty());
}

#[test]
fn steam_libraries_play_no_part_on_windows_inputs() {
    let site = site("steam_libraries_play_no_part_on_windows_inputs");
    let root = steam_root(&site, ".local/share/Steam");
    prefix(&root, &GAME_A, ID_1);

    let found = discover(&windows(Some(&site.home), Some(&site.home), Some(&site.home)), &[GAME_A]);

    assert!(found.folders.is_empty() && found.skipped.is_empty());
}

#[cfg(unix)]
#[test]
fn an_unreadable_game_folder_is_reported() {
    use neoffse_io::error::{IoError, Operation};

    let site = site("an_unreadable_game_folder_is_reported");
    let documents = site.base.join("docs");
    documents_save(&documents, &GAME_A, ID_1);
    let steam = steam_dir(&documents, &GAME_A);
    if !common::discovery::lock(&steam) {
        return;
    }

    let found = discover(&windows(Some(&documents), None, None), &[GAME_A]);

    common::discovery::unlock(&steam);
    assert_eq!(paths(&found), Vec::<PathBuf>::new());
    assert_eq!(found.skipped.len(), 1);
    let IoError::Io { operation, path, .. } = &found.skipped[0] else {
        panic!("unexpected error {:?}", found.skipped[0]);
    };
    assert_eq!((*operation, path), (Operation::ListDir, &steam));
}

#[test]
fn the_platform_environment_matches_the_system() {
    let environment = Environment::platform();
    let windows_fields = [&environment.documents, &environment.user_profile, &environment.one_drive];
    let linux_fields = [&environment.home, &environment.data_home];
    let (silent, active) = if cfg!(windows) {
        (linux_fields.to_vec(), windows_fields.to_vec())
    } else {
        (windows_fields.to_vec(), linux_fields.to_vec())
    };

    assert!(silent.iter().all(|field| field.is_none()));
    for path in active.into_iter().flatten() {
        assert!(path.is_absolute());
    }
}
