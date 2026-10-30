//! The FF I job map and the original sprites.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::core::{Game, TableKind};
use neoffse::data::{Language, tables_for};
use neoffse_app::assets::{AssetProvider, OriginalSprites, PortraitClass, class_of};

const JOB_NAMES: [&str; 12] = [
    "Warrior",
    "Thief",
    "Monk",
    "Red Mage",
    "White Mage",
    "Black Mage",
    "Knight",
    "Ninja",
    "Master",
    "Red Wizard",
    "White Wizard",
    "Black Wizard",
];

fn pixels(job: u32) -> Vec<u8> {
    let image = OriginalSprites.portrait(Game::Ff1Pr, job).unwrap();
    image.to_rgba8().unwrap().as_bytes().to_vec()
}

fn image_path(class: PortraitClass) -> std::path::PathBuf {
    neoffse_testkit::workspace_root()
        .join("assets")
        .join("portraits")
        .join("ff1-pr")
        .join(format!("{}.png", class.file_stem()))
}

#[test]
fn ff1_jobs_map_to_six_classes() {
    for (index, class) in PortraitClass::ALL.into_iter().enumerate() {
        let base = u32::try_from(index).unwrap() + 1;
        assert_eq!(class_of(Game::Ff1Pr, base), Some(class), "job {base}");
        assert_eq!(class_of(Game::Ff1Pr, base + 6), Some(class), "job {}", base + 6);
    }
}

#[test]
fn the_class_map_follows_the_generated_job_names() {
    let tables = tables_for(Game::Ff1Pr).unwrap();
    for (index, expected) in JOB_NAMES.into_iter().enumerate() {
        let id = u32::try_from(index).unwrap() + 1;
        let entry = tables.entry(TableKind::Job, id).unwrap();
        assert_eq!(entry.name(Language::En), Some(expected), "job {id}");
    }
}

#[test]
fn a_promoted_job_shares_its_base_image() {
    for base in 1..=6 {
        assert_eq!(pixels(base), pixels(base + 6), "job {base}");
    }
}

#[test]
fn the_six_base_images_differ() {
    let all: Vec<Vec<u8>> = (1..=6).map(pixels).collect();
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn no_portrait_for_job_zero_thirteen_or_u32_max() {
    for job in [0, 13, u32::MAX] {
        assert!(class_of(Game::Ff1Pr, job).is_none(), "job {job}");
        assert!(OriginalSprites.portrait(Game::Ff1Pr, job).is_none(), "job {job}");
    }
}

#[test]
fn another_game_has_no_portrait() {
    for job in 1..=12 {
        assert!(class_of(Game::Ff2Pr, job).is_none(), "job {job}");
        assert!(OriginalSprites.portrait(Game::Ff2Pr, job).is_none(), "job {job}");
    }
}

#[test]
fn the_embedded_bytes_equal_the_committed_files() {
    for class in PortraitClass::ALL {
        let path = image_path(class);
        let file = std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        assert_eq!(class.png(), file.as_slice(), "{}", class.file_stem());
    }
}

#[test]
fn a_portrait_is_32_px() {
    for job in 1..=12 {
        let size = OriginalSprites.portrait(Game::Ff1Pr, job).unwrap().size();
        assert_eq!((size.width, size.height), (32, 32), "job {job}");
    }
}

#[test]
fn file_stems_match_the_files() {
    for class in PortraitClass::ALL {
        assert!(image_path(class).is_file(), "{}", class.file_stem());
    }
}
