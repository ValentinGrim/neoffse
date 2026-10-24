//! The blocking scan: discovery, then the slots of every save folder.

use std::cmp::Ordering;
use std::time::Duration;

use neoffse::Registry;
use neoffse::core::{Game, SaveSlot, SaveSummary};
use neoffse::io::discovery::{Environment, Origin, SaveFolder, SteamGame, discover};
use neoffse::io::files;
use neoffse::pr::games::{GAMES, PrGame};
use neoffse::pr::slots::slot_for_name;

use super::{FolderData, GameData, HomeData, SlotData, SummaryData};
use crate::{FolderOrigin, SlotKind};

/// Runs discovery and reads every slot it finds. Blocking: the app calls it
/// on a worker thread.
///
/// Games come in `GAMES` order, folders in discovery order, slots numbered
/// first, then auto, then quick, then the rest by file name. Keys count the
/// slots in that order over the whole scan.
#[must_use]
pub fn scan_home(registry: &Registry, environment: &Environment) -> HomeData {
    let table: Vec<(Game, &PrGame)> = GAMES.iter().filter_map(|row| Game::from_id(row.id).map(|game| (game, row))).collect();
    let steam: Vec<SteamGame<'_>> = table
        .iter()
        .map(|(_, row)| SteamGame {
            id: row.id,
            app_id: row.app_id,
            folder: row.folder,
        })
        .collect();
    let discovery = discover(environment, &steam);

    let mut skipped = discovery.skipped.len();
    let mut next_key = 0_i32;
    let mut games = Vec::with_capacity(table.len());
    for (game, row) in table {
        let supported = registry.format(game).is_some();
        let found: Vec<&SaveFolder> = discovery.folders.iter().filter(|folder| folder.game == row.id).collect();
        let owned = !found.is_empty() || discovery.installs.iter().any(|install| install.game == row.id);
        let mut folders = Vec::with_capacity(found.len());
        for folder in found {
            let (mut slots, lost) = folder_slots(registry, game, supported, folder);
            skipped += lost;
            slots.sort_by(compare_slots);
            for slot in &mut slots {
                slot.key = next_key;
                next_key = next_key.saturating_add(1);
            }
            folders.push(FolderData {
                origin: origin_of(&folder.origin),
                dir: folder.path.clone(),
                slots,
            });
        }
        games.push(GameData {
            game,
            supported,
            owned,
            folders,
        });
    }
    HomeData { games, skipped }
}

fn origin_of(origin: &Origin) -> FolderOrigin {
    match origin {
        Origin::SteamLibrary { .. } => FolderOrigin::SteamProton,
        Origin::Documents { .. } => FolderOrigin::Documents,
        _ => FolderOrigin::Other,
    }
}

/// The slots of one folder, with the count of places that could not be read.
/// The keys are not set yet.
fn folder_slots(registry: &Registry, game: Game, supported: bool, folder: &SaveFolder) -> (Vec<SlotData>, usize) {
    if supported {
        supported_slots(registry, game, folder)
    } else {
        named_slots(folder)
    }
}

/// A game with a format: the registry claims the files, and each claimed file
/// opens once to read its summary.
fn supported_slots(registry: &Registry, game: Game, folder: &SaveFolder) -> (Vec<SlotData>, usize) {
    let Ok(scan) = registry.scan(&folder.path, Some(game)) else {
        return (Vec::new(), 1);
    };
    let skipped = scan.unreadable.len() + scan.non_utf8;
    let slots = scan
        .saves
        .into_iter()
        .map(|found| {
            let summary = registry
                .open(&folder.path, &found.name, Some(game))
                .ok()
                .and_then(|mut session| session.summary().ok());
            // The session drops here, before the next file opens.
            let (kind, number) = slot_kind_of(summary.as_ref(), &found.name);
            SlotData {
                key: 0,
                name: found.name,
                kind,
                number,
                unreadable: summary.is_none(),
                summary: summary.as_ref().map(summary_data),
                openable: true,
            }
        })
        .collect();
    (slots, skipped)
}

/// A game with no format: the file name alone tells a slot. The listing reads
/// no byte of any file.
fn named_slots(folder: &SaveFolder) -> (Vec<SlotData>, usize) {
    let Ok(listing) = files::list(&folder.path, 0) else {
        return (Vec::new(), 1);
    };
    let skipped = listing.unreadable.len() + listing.non_utf8;
    let slots = listing
        .files
        .into_iter()
        .filter_map(|file| {
            slot_for_name(&file.name)?;
            let (kind, number) = slot_kind_of(None, &file.name);
            Some(SlotData {
                key: 0,
                name: file.name,
                kind,
                number,
                summary: None,
                unreadable: false,
                openable: false,
            })
        })
        .collect();
    (slots, skipped)
}

/// The single rule for a slot's kind and number, shared by the Home rows and
/// the editor page: the summary's slot, else the one the file name gives,
/// else `Other`.
pub(crate) fn slot_kind_of(summary: Option<&SaveSummary>, name: &str) -> (SlotKind, i32) {
    slot_kind(summary.and_then(|summary| summary.slot).or_else(|| slot_for_name(name)))
}

fn slot_kind(slot: Option<SaveSlot>) -> (SlotKind, i32) {
    match slot {
        Some(SaveSlot::Numbered(number)) => (SlotKind::Numbered, i32::try_from(number).unwrap_or(i32::MAX)),
        Some(SaveSlot::Auto) => (SlotKind::AutoSave, 0),
        Some(SaveSlot::Quick) => (SlotKind::QuickSave, 0),
        _ => (SlotKind::Other, 0),
    }
}

fn rank(kind: SlotKind) -> u8 {
    match kind {
        SlotKind::Numbered => 0,
        SlotKind::AutoSave => 1,
        SlotKind::QuickSave => 2,
        SlotKind::Other => 3,
    }
}

/// Numbered ascending, auto, quick, then the rest; ties by file name in byte
/// order, so the listing order never shows.
fn compare_slots(a: &SlotData, b: &SlotData) -> Ordering {
    rank(a.kind)
        .cmp(&rank(b.kind))
        .then(a.number.cmp(&b.number))
        .then_with(|| a.name.as_bytes().cmp(b.name.as_bytes()))
}

pub(super) fn summary_data(summary: &SaveSummary) -> SummaryData {
    SummaryData {
        party: summary.party.join(", "),
        location: summary.location.clone().unwrap_or_default(),
        play_time: summary.play_time.map(play_time_text).unwrap_or_default(),
    }
}

/// `H:MM:SS`, sub-seconds dropped.
fn play_time_text(time: Duration) -> String {
    let seconds = time.as_secs();
    format!("{}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
}
