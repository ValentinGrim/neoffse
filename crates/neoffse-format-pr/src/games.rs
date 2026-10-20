//! The six Pixel Remaster games on Steam as plain data: NeoFFSE game id,
//! Steam app ID, folder name under `My Games` and executable name.
//! `docs/formats/pixel-remaster.md#games` is the source of every value.
//!
//! The `GAMES` table reads no file and holds no type of another crate. A
//! caller copies `id`, `app_id` and `folder` of a row into the I/O crate's
//! game descriptor for save discovery, and matches `executable` against the
//! running processes.
//!
//! [`identify`] tells the game of a slot document by its vehicle count and
//! a few marker keys, the rule every PR format applies when it opens a file.

use neoffse_core::Game;

use crate::document::{Document, DocumentError, Segment};
use crate::open_error::OpenError;

/// One Pixel Remaster game on Steam.
///
/// FF I has no "I" in its folder and executable names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PrGame {
    /// The NeoFFSE game id: `ff1-pr` to `ff6-pr`. It names the backup folder, so it
    /// holds only ASCII lowercase letters, digits and `-`.
    pub id: &'static str,
    /// The Steam app ID.
    pub app_id: u32,
    /// The folder name under `My Games`, which is also the install folder
    /// under `steamapps/common/`. One plain folder name.
    pub folder: &'static str,
    /// The executable file name, as the game runs under Windows, extension included.
    pub executable: &'static str,
}

/// The six games, FF I first.
///
/// A caller builds the I/O crate's game descriptor from `id`, `app_id` and
/// `folder`; `executable` feeds the running-game check.
pub const GAMES: [PrGame; 6] = [
    PrGame {
        id: "ff1-pr",
        app_id: 1_173_770,
        folder: "FINAL FANTASY PR",
        executable: "FINAL FANTASY.exe",
    },
    PrGame {
        id: "ff2-pr",
        app_id: 1_173_780,
        folder: "FINAL FANTASY II PR",
        executable: "FINAL FANTASY II.exe",
    },
    PrGame {
        id: "ff3-pr",
        app_id: 1_173_790,
        folder: "FINAL FANTASY III PR",
        executable: "FINAL FANTASY III.exe",
    },
    PrGame {
        id: "ff4-pr",
        app_id: 1_173_800,
        folder: "FINAL FANTASY IV PR",
        executable: "FINAL FANTASY IV.exe",
    },
    PrGame {
        id: "ff5-pr",
        app_id: 1_173_810,
        folder: "FINAL FANTASY V PR",
        executable: "FINAL FANTASY V.exe",
    },
    PrGame {
        id: "ff6-pr",
        app_id: 1_173_820,
        folder: "FINAL FANTASY VI PR",
        executable: "FINAL FANTASY VI.exe",
    },
];

/// Vehicle counts of the six games, as `userData.ownedTransportationList.target`
/// holds them (`docs/formats/pixel-remaster.md#telling-the-games-apart`).
/// Every count is a reading of the saves of one Steam build [M]; the FF I
/// count also matches the game's master data.
const VEHICLE_COUNTS: [(usize, Game); 6] = [
    (4, Game::Ff1Pr),
    (7, Game::Ff2Pr),
    (14, Game::Ff3Pr),
    (11, Game::Ff4Pr),
    (19, Game::Ff5Pr),
    (8, Game::Ff6Pr),
];

const VEHICLES: [Segment<'static>; 5] = [
    Segment::Key("userData"),
    Segment::Doc,
    Segment::Key("ownedTransportationList"),
    Segment::Doc,
    Segment::Key("target"),
];

/// Keys that only one game holds, so their presence contradicts the count of
/// another game (`docs/formats/pixel-remaster.md#telling-the-games-apart`,
/// step 4). `cheatSettingsData` is a plain nested object, so its path has no
/// `Doc` step. A game's own marker is never required.
const MARKERS: [(Game, &[Segment<'static>]); 3] = [
    (
        Game::Ff3Pr,
        &[
            Segment::Key("userData"),
            Segment::Doc,
            Segment::Key("cheatSettingsData"),
            Segment::Key("jobProficiencyRate"),
        ],
    ),
    (Game::Ff5Pr, &[Segment::Key("userData"), Segment::Doc, Segment::Key("wonderWandIndex")]),
    (Game::Ff6Pr, &[Segment::Key("mapData"), Segment::Doc, Segment::Key("otherPartyDataList")]),
];

/// Identifies the game of a slot document by the rule of
/// `docs/formats/pixel-remaster.md#telling-the-games-apart` (steps 1 to 4):
/// the vehicle count names the game, then a marker key of another game
/// refuses the document.
///
/// The call reads only: the document text stays byte for byte.
///
/// # Errors
///
/// `OpenError::NotASlot` when `userData` is absent or holds no JSON object,
/// or when `ownedTransportationList` or its `target` array is missing or of
/// another kind, or when a marker path fails for any reason but an absent key.
/// `OpenError::OtherGame { found: None, .. }` for a count no game has, and
/// `found: Some(game)` when a marker key contradicts the count.
pub fn identify(document: &mut Document) -> Result<Game, OpenError> {
    identify_counted(document).map(|(game, _)| game)
}

/// [`identify`] with the vehicle count, for a caller that reports it.
pub(crate) fn identify_counted(document: &mut Document) -> Result<(Game, usize), OpenError> {
    let vehicles = document.array_len(&VEHICLES).map_err(not_a_slot)?;
    let Some(&(_, game)) = VEHICLE_COUNTS.iter().find(|(count, _)| *count == vehicles) else {
        return Err(OpenError::OtherGame { found: None, vehicles });
    };
    for (marker_game, path) in MARKERS {
        if marker_game != game && holds(document, path)? {
            return Err(OpenError::OtherGame {
                found: Some(marker_game),
                vehicles,
            });
        }
    }
    Ok((game, vehicles))
}

/// Whether `path` exists. An absent key is `false`; any other failure is not
/// a slot.
fn holds(document: &mut Document, path: &[Segment<'_>]) -> Result<bool, OpenError> {
    match document.raw(path) {
        Ok(_) => Ok(true),
        Err(DocumentError::MissingKey { .. }) => Ok(false),
        Err(error) => Err(not_a_slot(error)),
    }
}

fn not_a_slot(error: DocumentError) -> OpenError {
    OpenError::NotASlot { source: Box::new(error) }
}
