//! The scan and the rows it produces: grouping, labels, order, keys and
//! support. Every folder sits under a temporary directory.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::fs;

use common::{ID_1, ID_2, Site, all_rows, documents_slot, environment, items, registry, scanned, slot_bytes, slot_file, slots_of, steam_slot};
use neoffse::core::Game;
use neoffse::pr::games::GAMES;
use neoffse_app::home::{HomeData, HomeViewModel, scan_home};
use neoffse_app::restore::same_folder;
use neoffse_app::{FolderOrigin, GameRow, SlotKind, SlotRow};
use slint::Model;

const FF1: usize = 0;
const FF2: usize = 1;

fn all_slots(row: &GameRow) -> Vec<SlotRow> {
    let mut slots = slots_of(row);
    for folder in items(&row.folders) {
        slots.extend(items(&folder.slots));
    }
    slots
}

#[test]
fn a_proton_save_makes_one_ungrouped_slot() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let rows = all_rows(&scanned(&site));

    let titles: Vec<String> = rows.iter().map(|row| row.title.to_string()).collect();
    let expected = [
        "Final Fantasy I",
        "Final Fantasy II",
        "Final Fantasy III",
        "Final Fantasy IV",
        "Final Fantasy V",
        "Final Fantasy VI",
    ];
    assert_eq!(titles, expected);
    assert_eq!(rows.len(), GAMES.len());

    let ff1 = &rows[0];
    assert!(ff1.supported);
    assert!(!ff1.no_save);
    assert_eq!(ff1.folders.row_count(), 0);
    let slots = slots_of(ff1);
    assert_eq!(slots.len(), 1);
    let slot = &slots[0];
    assert!(slot.summarized && slot.openable && !slot.unreadable);
    assert_eq!(slot.summary.kind, SlotKind::Numbered);
    assert_eq!(slot.summary.number, 1);
    assert_eq!(slot.summary.game.as_str(), "Final Fantasy I");
    assert_eq!(slot.summary.party.as_str(), "Aria, Bram, Cleo, Zoé");
    assert_eq!(slot.summary.location.as_str(), "Invented Plains");
    assert_eq!(slot.summary.play_time.as_str(), "3:47:01");
}

#[test]
fn two_origins_make_two_labelled_folders() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    documents_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let rows = scanned(&site).rows();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].slots.row_count(), 0);
    let folders = items(&rows[0].folders);
    let origins: Vec<FolderOrigin> = folders.iter().map(|folder| folder.origin).collect();
    assert_eq!(origins, [FolderOrigin::SteamProton, FolderOrigin::Documents]);
    let accounts: Vec<i32> = folders.iter().map(|folder| folder.account).collect();
    assert_eq!(accounts, [1, 2]);
    assert!(folders.iter().all(|folder| folder.slots.row_count() == 1));
}

#[test]
fn two_accounts_in_one_prefix_make_two_folders() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_2, slot_file(2), &slot_bytes(2));
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let rows = scanned(&site).rows();

    let folders = items(&rows[0].folders);
    assert_eq!(folders.len(), 2);
    assert!(folders.iter().all(|folder| folder.origin == FolderOrigin::SteamProton));
    assert_eq!(folders.iter().map(|folder| folder.account).collect::<Vec<_>>(), [1, 2]);
    // Ids ascend: the first account holds the slot written for the lower id.
    let numbers: Vec<i32> = folders.iter().map(|folder| items(&folder.slots)[0].summary.number).collect();
    assert_eq!(numbers, [1, 2]);
}

#[test]
fn a_single_folder_shows_no_account() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let rows = scanned(&site).rows();

    assert_eq!(rows[0].folders.row_count(), 0);
    assert_eq!(rows[0].slots.row_count(), 1);
}

#[test]
fn no_string_holds_an_id_or_a_path() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    steam_slot(&site, FF1, ID_2, slot_file(2), &slot_bytes(2));
    let rows = all_rows(&scanned(&site));
    let root = site.root().to_string_lossy().into_owned();

    let mut strings = Vec::new();
    for row in &rows {
        strings.push(row.title.to_string());
        for slot in all_slots(row) {
            strings.extend([
                slot.summary.game.to_string(),
                slot.summary.party.to_string(),
                slot.summary.location.to_string(),
                slot.summary.play_time.to_string(),
            ]);
        }
    }
    assert!(strings.len() > 10);
    for text in &strings {
        for secret in [ID_1, ID_2, root.as_str(), slot_file(1), slot_file(2)] {
            assert!(!text.contains(secret), "a row string leaks a private value");
        }
    }
}

#[test]
fn slots_sort_numbered_then_auto_then_quick() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(22), &slot_bytes(22));
    steam_slot(&site, FF1, ID_1, slot_file(3), &slot_bytes(3));
    steam_slot(&site, FF1, ID_1, slot_file(21), &slot_bytes(21));
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let slots = slots_of(&scanned(&site).rows()[0]);

    let kinds: Vec<(SlotKind, i32)> = slots.iter().map(|slot| (slot.summary.kind, slot.summary.number)).collect();
    assert_eq!(
        kinds,
        [
            (SlotKind::Numbered, 1),
            (SlotKind::Numbered, 3),
            (SlotKind::AutoSave, 0),
            (SlotKind::QuickSave, 0)
        ]
    );
}

#[test]
fn a_summary_slot_wins_over_the_file_name() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(3), &slot_bytes(2));
    let slots = slots_of(&scanned(&site).rows()[0]);

    assert_eq!(slots[0].summary.kind, SlotKind::Numbered);
    assert_eq!(slots[0].summary.number, 2);
}

#[test]
fn an_unreadable_slot_is_listed_and_flagged() {
    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(2), b"not a save");
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let slots = slots_of(&scanned(&site).rows()[0]);

    assert_eq!(slots.len(), 2);
    let good = &slots[0];
    assert!(good.summarized && !good.unreadable);
    let bad = &slots[1];
    assert!(bad.unreadable && !bad.summarized && bad.openable);
    assert_eq!(bad.summary.kind, SlotKind::Numbered);
    assert_eq!(bad.summary.number, 2);
}

#[test]
fn a_game_without_a_format_lists_slots_but_cannot_open_them() {
    let site = Site::new();
    let folder = steam_slot(&site, FF2, ID_1, slot_file(1), b"opaque");
    fs::write(folder.join("stray.txt"), b"x").unwrap();
    fs::write(folder.join(".neoffse-x.tmp"), b"x").unwrap();
    let rows = scanned(&site).rows();

    assert_eq!(rows.len(), 1);
    let ff2 = &rows[0];
    assert!(!ff2.supported);
    let slots = slots_of(ff2);
    assert_eq!(slots.len(), 1);
    assert!(!slots[0].summarized && !slots[0].openable && !slots[0].unreadable);
    assert_eq!(slots[0].summary.kind, SlotKind::Numbered);
    assert_eq!(slots[0].summary.number, 1);
}

#[test]
fn support_comes_from_the_registry() {
    let site = Site::new();
    for index in 0..GAMES.len() {
        steam_slot(&site, index, ID_1, slot_file(1), &slot_bytes(1));
    }
    let rows = all_rows(&scanned(&site));
    let registry = registry();

    assert_eq!(rows.len(), GAMES.len());
    for (row, game) in rows.iter().zip(GAMES.iter()) {
        let game = Game::from_id(game.id).unwrap();
        assert_eq!(row.supported, registry.format(game).is_some(), "{}", row.title);
    }
}

#[test]
fn keys_are_unique_and_resolve_to_targets() {
    let site = Site::new();
    let first = steam_slot(&site, FF1, ID_1, slot_file(2), &slot_bytes(2));
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let second = documents_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let third = steam_slot(&site, FF2, ID_1, slot_file(3), b"opaque");
    let folders = [first, second, third.clone()];
    let home = scanned(&site);
    let rows = all_rows(&home);

    let slots: Vec<SlotRow> = rows.iter().flat_map(all_slots).collect();
    assert_eq!(slots.len(), 4);
    let mut keys: Vec<i32> = slots.iter().map(|slot| slot.key).collect();
    keys.sort_unstable();
    keys.dedup();
    assert_eq!(keys.len(), 4);

    for slot in &slots {
        let target = home.slot_target(slot.key).expect("a listed key resolves");
        assert_eq!(target.name, slot_file(u32::try_from(slot.summary.number).unwrap()));
        assert_eq!(target.supported, slot.openable);
        assert!(target.dir.join(&target.name).is_file(), "the target names a file of its folder");
        assert!(
            folders.iter().any(|folder| same_folder(folder, &target.dir)),
            "the target folder is one of the scanned folders"
        );
        assert_eq!(target.game == Game::Ff2Pr, same_folder(&target.dir, &third));
    }
    assert!(home.slot_target(-1).is_none());
    assert!(home.slot_target(4).is_none());
}

#[test]
fn an_empty_environment_finds_nothing() {
    let home = HomeViewModel::new(registry(), neoffse::io::discovery::Environment::default());
    let manual = common::Manual::new();
    home.start_scan(&manual.dispatcher(), || {});
    manual.run_all();

    assert_eq!(home.rows().len(), 0);
    let rows = all_rows(&home);
    assert_eq!(rows.len(), GAMES.len());
    assert!(rows[0].supported && rows[0].no_save);
    for row in &rows[1..] {
        assert!(!row.supported && !row.no_save, "{}", row.title);
    }
}

#[test]
fn a_scan_can_run_without_the_view_model() {
    fn needs_send<T: Send>() {}
    fn needs_sync<T: Sync>() {}
    needs_send::<HomeData>();
    needs_send::<HomeViewModel>();
    needs_sync::<HomeViewModel>();

    let site = Site::new();
    steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    let data: HomeData = scan_home(&registry(), &environment(&site));
    assert!(format!("{data:?}").contains("Aria"), "Debug prints save content, as the rustdoc warns");
}

#[cfg(unix)]
#[test]
fn a_non_utf8_name_counts_as_skipped() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let site = Site::new();
    let folder = steam_slot(&site, FF1, ID_1, slot_file(1), &slot_bytes(1));
    fs::write(folder.join(OsStr::from_bytes(b"bad-\xff-name")), b"x").unwrap();
    let home = scanned(&site);

    assert_eq!(slots_of(&home.rows()[0]).len(), 1);
    assert!(home.skipped_count() >= 1);
}
