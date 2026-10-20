//! The error of opening a file as a slot of one game.
//!
//! The type lives here, not in [`crate::ff1`], so that [`crate::games`] can
//! return it without a cycle between the two modules.

use neoffse_core::Game;

/// Why a file is not a slot of the game that tried to open it.
///
/// `FormatError::Decode` carries it as its source. Each variant follows the
/// rule of `docs/formats/pixel-remaster.md#telling-the-games-apart`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenError {
    /// The set held no file or several. A Pixel Remaster save is one file.
    #[error("an FF I save is one file, found {found}")]
    FileCount {
        /// The number of files in the set.
        found: usize,
    },
    /// The file decodes but holds no slot: the bestiary, the slot index and
    /// the system settings fail here, and so does a slot without its
    /// characters or item lists.
    #[error("the file holds no FF I slot")]
    NotASlot {
        /// The `DocumentError` or `EditError` that failed the game rule or the
        /// shape check.
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// The file is a slot, but of another game.
    #[error("the file is a slot of another game ({vehicles} vehicles)")]
    OtherGame {
        /// The game the vehicle count or a marker key names. `None` when the
        /// count belongs to no game, for instance after a build changed a
        /// transportation table.
        found: Option<Game>,
        /// The entry count of `userData.ownedTransportationList.target`.
        vehicles: usize,
    },
}
