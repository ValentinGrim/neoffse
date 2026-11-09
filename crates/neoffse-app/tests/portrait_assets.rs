//! Checks the class portraits under `assets/portraits/ff1-pr/` and their
//! licence. Reads repository files only.

use slint::{Rgba8Pixel, SharedPixelBuffer};

mod asset_files;

use asset_files::{assert_portrait_png, pixels_in, portraits_dir, read_text};

/// The six FF I classes that have a sprite, as file stems, with the number of
/// opaque pixels each image holds.
const CLASSES: [(&str, usize); 6] = [
    ("warrior", 672),
    ("thief", 505),
    ("monk", 591),
    ("red-mage", 595),
    ("white-mage", 694),
    ("black-mage", 663),
];

fn pixels(stem: &str) -> SharedPixelBuffer<Rgba8Pixel> {
    pixels_in("ff1-pr", stem)
}

#[test]
fn every_image_is_a_32_px_rgba_png_with_binary_alpha_and_clear_corners() {
    for (stem, _) in CLASSES {
        assert_portrait_png("ff1-pr", stem);
    }
}

#[test]
fn each_image_has_its_pinned_opaque_count() {
    for (stem, expected) in CLASSES {
        let opaque = pixels(stem).as_slice().iter().filter(|pixel| pixel.a == 255).count();
        assert_eq!(opaque, expected, "{stem}.png opaque pixel count");
    }
}

#[test]
fn the_six_images_differ_pairwise() {
    let all: Vec<_> = CLASSES.iter().map(|(stem, _)| pixels(stem)).collect();
    for (i, first) in all.iter().enumerate() {
        for (j, second) in all.iter().enumerate().skip(i + 1) {
            assert_ne!(first.as_slice(), second.as_slice(), "{} and {} are the same image", CLASSES[i].0, CLASSES[j].0);
        }
    }
}

#[test]
fn the_licence_is_a_cc0_dedication() {
    let notice = read_text(&portraits_dir().join("LICENSE"));
    for needle in [
        "CC0 1.0",
        "https://creativecommons.org/publicdomain/zero/1.0/",
        "https://creativecommons.org/publicdomain/zero/1.0/legalcode",
        "waived all copyright",
    ] {
        assert!(notice.contains(needle), "assets/portraits/LICENSE lacks {needle:?}");
    }
}

#[test]
fn the_readme_lists_the_portraits() {
    let readme = read_text(&neoffse_testkit::workspace_root().join("licenses").join("README.md"));
    for needle in ["assets/portraits", "CC0 1.0"] {
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
