//! The FF II person map and the original sprites that follow it.

#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse::core::{Game, TableKind};
use neoffse::data::{Language, tables_for};
use neoffse_app::assets::{AssetProvider, OriginalSprites, PortraitPerson, person_of};

const PEOPLE: [&str; 9] = ["Firion", "Maria", "Guy", "Minwu", "Josef", "Gordon", "Leila", "Ricard", "Leon"];

fn pixels(person: PortraitPerson) -> Vec<u8> {
    let image = slint::Image::load_from_data(person.png(), Some("png")).unwrap();
    image.to_rgba8().unwrap().as_bytes().to_vec()
}

fn image_path(person: PortraitPerson) -> std::path::PathBuf {
    neoffse_testkit::workspace_root()
        .join("assets")
        .join("portraits")
        .join("ff2-pr")
        .join(format!("{}.png", person.file_stem()))
}

#[test]
fn ff2_jobs_one_to_nine_map_in_order() {
    for (index, person) in PortraitPerson::ALL.into_iter().enumerate() {
        let job = u32::try_from(index).unwrap() + 1;
        assert_eq!(person_of(Game::Ff2Pr, job), Some(person), "job {job}");
    }
}

#[test]
fn no_person_for_job_zero_ten_or_u32_max() {
    for job in [0, 10, u32::MAX] {
        assert!(person_of(Game::Ff2Pr, job).is_none(), "job {job}");
        assert!(OriginalSprites.portrait(Game::Ff2Pr, job).is_none(), "job {job}");
    }
}

#[test]
fn another_game_has_no_person() {
    for game in [Game::Ff1Pr, Game::Ff3Pr] {
        for job in 1..=12 {
            assert!(person_of(game, job).is_none(), "{game:?} job {job}");
        }
    }
}

#[test]
fn the_order_follows_the_generated_job_names() {
    let tables = tables_for(Game::Ff2Pr).unwrap();
    for (index, expected) in PEOPLE.into_iter().enumerate() {
        let id = u32::try_from(index).unwrap() + 1;
        let entry = tables.entry(TableKind::Job, id).unwrap();
        assert_eq!(entry.name(Language::En), Some(expected), "job {id}");
        let person = person_of(Game::Ff2Pr, id).unwrap_or_else(|| panic!("job {id} has no person"));
        assert_eq!(person.file_stem(), expected.to_lowercase(), "job {id}");
    }
}

#[test]
fn the_embedded_bytes_equal_the_committed_files() {
    for person in PortraitPerson::ALL {
        let path = image_path(person);
        let file = std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        assert_eq!(person.png(), file.as_slice(), "{}", person.file_stem());
    }
}

#[test]
fn a_portrait_is_32_px() {
    for job in 1..=9 {
        let size = OriginalSprites.portrait(Game::Ff2Pr, job).unwrap().size();
        assert_eq!((size.width, size.height), (32, 32), "job {job}");
    }
}

#[test]
fn the_provider_gives_the_image_of_the_person() {
    for (index, person) in PortraitPerson::ALL.into_iter().enumerate() {
        let job = u32::try_from(index).unwrap() + 1;
        let image = OriginalSprites.portrait(Game::Ff2Pr, job).unwrap();
        assert!(image.to_rgba8().unwrap().as_bytes() == pixels(person).as_slice(), "job {job}");
    }
}

#[test]
fn the_nine_people_differ() {
    let all: Vec<Vec<u8>> = PortraitPerson::ALL.into_iter().map(pixels).collect();
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn file_stems_match_the_files() {
    for person in PortraitPerson::ALL {
        assert!(image_path(person).is_file(), "{}", person.file_stem());
    }
}
