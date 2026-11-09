//! Checks the person portraits under `assets/portraits/ff2-pr/` and their
//! licence. Reads repository files only.

use slint::{Rgba8Pixel, SharedPixelBuffer};

mod asset_files;

use asset_files::{assert_portrait_png, pixels_in, portraits_dir, read_text};

/// The nine FF II people as file stems, in `job.csv` order, with the number of
/// opaque pixels each image holds.
const PEOPLE: [(&str, usize); 9] = [
    ("firion", 620),
    ("maria", 602),
    ("guy", 583),
    ("minwu", 682),
    ("josef", 598),
    ("gordon", 605),
    ("leila", 586),
    ("ricard", 570),
    ("leon", 561),
];

/// The six FF I class stems under `ff1-pr/`.
const CLASSES: [&str; 6] = ["warrior", "thief", "monk", "red-mage", "white-mage", "black-mage"];

fn pixels(stem: &str) -> SharedPixelBuffer<Rgba8Pixel> {
    pixels_in("ff2-pr", stem)
}

#[test]
fn every_image_is_a_32_px_rgba_png_with_binary_alpha_and_clear_corners() {
    for (stem, _) in PEOPLE {
        assert_portrait_png("ff2-pr", stem);
    }
}

#[test]
fn each_image_has_its_pinned_opaque_count() {
    for (stem, expected) in PEOPLE {
        let opaque = pixels(stem).as_slice().iter().filter(|pixel| pixel.a == 255).count();
        assert_eq!(opaque, expected, "{stem}.png opaque pixel count");
    }
}

#[test]
fn the_nine_images_differ_pairwise() {
    let all: Vec<_> = PEOPLE.iter().map(|(stem, _)| pixels(stem)).collect();
    for (i, first) in all.iter().enumerate() {
        for (j, second) in all.iter().enumerate().skip(i + 1) {
            assert_ne!(first.as_slice(), second.as_slice(), "{} and {} are the same image", PEOPLE[i].0, PEOPLE[j].0);
        }
    }
}

#[test]
fn the_nine_images_differ_from_the_six_class_images() {
    for (stem, _) in PEOPLE {
        let person = pixels(stem);
        for class in CLASSES {
            let class_pixels = pixels_in("ff1-pr", class);
            assert_ne!(person.as_slice(), class_pixels.as_slice(), "{stem} and {class} are the same image");
        }
    }
}

#[test]
fn the_licence_is_a_cc0_dedication_for_both_folders() {
    let notice = read_text(&portraits_dir().join("LICENSE"));
    for needle in [
        "CC0 1.0",
        "https://creativecommons.org/publicdomain/zero/1.0/",
        "https://creativecommons.org/publicdomain/zero/1.0/legalcode",
        "waived all copyright",
        "ff1-pr",
        "ff2-pr",
    ] {
        assert!(notice.contains(needle), "assets/portraits/LICENSE lacks {needle:?}");
    }
}

#[test]
fn the_readme_lists_both_portrait_folders() {
    let readme = read_text(&neoffse_testkit::workspace_root().join("licenses").join("README.md"));
    for needle in ["assets/portraits/ff1-pr", "assets/portraits/ff2-pr", "CC0 1.0"] {
        assert!(readme.contains(needle), "licenses/README.md lacks {needle}");
    }
}

#[test]
fn the_repository_files_name_no_scratch_path() {
    let notice = read_text(&portraits_dir().join("LICENSE"));
    assert!(!notice.contains(".scratch"), "assets/portraits/LICENSE names a scratch path");
    let readme = read_text(&neoffse_testkit::workspace_root().join("licenses").join("README.md"));
    let section = readme
        .split("## Portraits")
        .nth(1)
        .unwrap_or_else(|| panic!("licenses/README.md has no Portraits section"));
    assert!(!section.contains(".scratch"), "the Portraits section names a scratch path");
}
