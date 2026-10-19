//! The six Pixel Remaster games on Steam as plain data: NeoFFSE game id,
//! Steam app ID, folder name under `My Games` and executable name.
//! `docs/formats/pixel-remaster.md#games` is the source of every value.
//!
//! The module reads no file and calls no other crate. A caller copies `id`,
//! `app_id` and `folder` of a row into the I/O crate's game descriptor for
//! save discovery, and matches `executable` against the running processes.

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
