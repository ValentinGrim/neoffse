//! The original FF II person sprites and the job map that picks one.
//!
//! The nine 32 by 32 PNG files are the project's own drawings, dedicated to
//! the public domain (`assets/portraits/LICENSE`). FF I keys are classes
//! ([`super::PortraitClass`], [`super::class_of`]); FF II keys are people.
//! Each game owns its key type and its map, so a per-job FF I map or FF III's
//! jobs add a variant or a map beside these without a change to a caller.
//!
//! In FF II the `jobId` of a character names the person: `job.csv` has nine
//! rows, one per person, and `jobId` equals the `job_id` of the status row
//! (`docs/formats/pixel-remaster.md#people-and-party-in-ff-ii`). Leon's two
//! forms (status 9 and 10) share job 9 and one image.

use neoffse::core::Game;

/// The nine people of FF II that have a sprite, in `job.csv` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortraitPerson {
    /// Job 1.
    Firion,
    /// Job 2.
    Maria,
    /// Job 3.
    Guy,
    /// Job 4.
    Minwu,
    /// Job 5.
    Josef,
    /// Job 6.
    Gordon,
    /// Job 7.
    Leila,
    /// Job 8.
    Ricard,
    /// Job 9, in both of his forms.
    Leon,
}

impl PortraitPerson {
    /// Every person, in job id order: `ALL[job - 1]`.
    pub const ALL: [PortraitPerson; 9] = [
        PortraitPerson::Firion,
        PortraitPerson::Maria,
        PortraitPerson::Guy,
        PortraitPerson::Minwu,
        PortraitPerson::Josef,
        PortraitPerson::Gordon,
        PortraitPerson::Leila,
        PortraitPerson::Ricard,
        PortraitPerson::Leon,
    ];

    /// The file name of the image without its extension, under
    /// `assets/portraits/ff2-pr/`.
    #[must_use]
    pub fn file_stem(self) -> &'static str {
        match self {
            PortraitPerson::Firion => "firion",
            PortraitPerson::Maria => "maria",
            PortraitPerson::Guy => "guy",
            PortraitPerson::Minwu => "minwu",
            PortraitPerson::Josef => "josef",
            PortraitPerson::Gordon => "gordon",
            PortraitPerson::Leila => "leila",
            PortraitPerson::Ricard => "ricard",
            PortraitPerson::Leon => "leon",
        }
    }

    /// The PNG file, embedded at build time.
    #[must_use]
    pub fn png(self) -> &'static [u8] {
        match self {
            PortraitPerson::Firion => include_bytes!("../../../../assets/portraits/ff2-pr/firion.png"),
            PortraitPerson::Maria => include_bytes!("../../../../assets/portraits/ff2-pr/maria.png"),
            PortraitPerson::Guy => include_bytes!("../../../../assets/portraits/ff2-pr/guy.png"),
            PortraitPerson::Minwu => include_bytes!("../../../../assets/portraits/ff2-pr/minwu.png"),
            PortraitPerson::Josef => include_bytes!("../../../../assets/portraits/ff2-pr/josef.png"),
            PortraitPerson::Gordon => include_bytes!("../../../../assets/portraits/ff2-pr/gordon.png"),
            PortraitPerson::Leila => include_bytes!("../../../../assets/portraits/ff2-pr/leila.png"),
            PortraitPerson::Ricard => include_bytes!("../../../../assets/portraits/ff2-pr/ricard.png"),
            PortraitPerson::Leon => include_bytes!("../../../../assets/portraits/ff2-pr/leon.png"),
        }
    }
}

/// The person of a job, `None` for another game or an id outside 1 to 9.
///
/// Another game's job 1 is not Firion: only FF II maps.
#[must_use]
pub fn person_of(game: Game, job: u32) -> Option<PortraitPerson> {
    if game != Game::Ff2Pr {
        return None;
    }
    // `u32` fits `usize` on every supported target; job 0 and past 9 are the documented `None`.
    let position = usize::try_from(job.checked_sub(1)?).ok()?;
    PortraitPerson::ALL.get(position).copied()
}
