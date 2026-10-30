//! The original FF I class sprites and the job map that picks one.
//!
//! The six 32 by 32 PNG files are the project's own drawings, dedicated to
//! the public domain (`assets/portraits/LICENSE`). A promoted job shares the
//! image of its base class: job N and job N + 6 belong to one character
//! (`docs/formats/pixel-remaster.md#ability-job-and-command-tables` lists the
//! twelve jobs, `#maximum-hp-and-experience-tables` pairs them).

use neoffse::core::Game;

use super::AssetProvider;

/// The six FF I classes that have a sprite. A promoted job shares the image
/// of its base class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PortraitClass {
    /// Job 1, promoted to the Knight (job 7).
    Warrior,
    /// Job 2, promoted to the Ninja (job 8).
    Thief,
    /// Job 3, promoted to the Master (job 9).
    Monk,
    /// Job 4, promoted to the Red Wizard (job 10).
    RedMage,
    /// Job 5, promoted to the White Wizard (job 11).
    WhiteMage,
    /// Job 6, promoted to the Black Wizard (job 12).
    BlackMage,
}

impl PortraitClass {
    /// Every class, in job id order.
    pub const ALL: [PortraitClass; 6] = [
        PortraitClass::Warrior,
        PortraitClass::Thief,
        PortraitClass::Monk,
        PortraitClass::RedMage,
        PortraitClass::WhiteMage,
        PortraitClass::BlackMage,
    ];

    /// The file name of the image without its extension, under
    /// `assets/portraits/ff1-pr/`.
    #[must_use]
    pub fn file_stem(self) -> &'static str {
        match self {
            PortraitClass::Warrior => "warrior",
            PortraitClass::Thief => "thief",
            PortraitClass::Monk => "monk",
            PortraitClass::RedMage => "red-mage",
            PortraitClass::WhiteMage => "white-mage",
            PortraitClass::BlackMage => "black-mage",
        }
    }

    /// The PNG file, embedded at build time.
    #[must_use]
    pub fn png(self) -> &'static [u8] {
        match self {
            PortraitClass::Warrior => include_bytes!("../../../../assets/portraits/ff1-pr/warrior.png"),
            PortraitClass::Thief => include_bytes!("../../../../assets/portraits/ff1-pr/thief.png"),
            PortraitClass::Monk => include_bytes!("../../../../assets/portraits/ff1-pr/monk.png"),
            PortraitClass::RedMage => include_bytes!("../../../../assets/portraits/ff1-pr/red-mage.png"),
            PortraitClass::WhiteMage => include_bytes!("../../../../assets/portraits/ff1-pr/white-mage.png"),
            PortraitClass::BlackMage => include_bytes!("../../../../assets/portraits/ff1-pr/black-mage.png"),
        }
    }
}

/// The class of a job, `None` for another game or an id outside 1 to 12.
///
/// Another game's job 1 is not the Warrior: FF III jobs are not FF I
/// classes, so no other game maps.
#[must_use]
pub fn class_of(game: Game, job: u32) -> Option<PortraitClass> {
    if game != Game::Ff1Pr {
        return None;
    }
    match job {
        1 | 7 => Some(PortraitClass::Warrior),
        2 | 8 => Some(PortraitClass::Thief),
        3 | 9 => Some(PortraitClass::Monk),
        4 | 10 => Some(PortraitClass::RedMage),
        5 | 11 => Some(PortraitClass::WhiteMage),
        6 | 12 => Some(PortraitClass::BlackMage),
        _ => None,
    }
}

/// The sprites drawn for the project (`assets/portraits/LICENSE`).
///
/// It decodes the PNG on each call and keeps nothing: [`slint::Image`] is not
/// `Send`, and the provider must stay `Send + Sync`.
#[derive(Debug, Clone, Copy, Default)]
pub struct OriginalSprites;

impl AssetProvider for OriginalSprites {
    fn portrait(&self, game: Game, job: u32) -> Option<slint::Image> {
        let class = class_of(game, job)?;
        // The PNGs are compiled in and `a_portrait_is_32_px` decodes each one: a failure is a build defect,
        // and the card falls back to its initial.
        slint::Image::load_from_data(class.png(), Some("png")).ok()
    }
}
