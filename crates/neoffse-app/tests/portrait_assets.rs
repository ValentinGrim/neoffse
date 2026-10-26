//! Checks the class portraits under `assets/portraits/ff1-pr/` and their
//! licence. Reads repository files only.

use std::fs;
use std::path::PathBuf;

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

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

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

fn portraits_dir() -> PathBuf {
    neoffse_testkit::workspace_root().join("assets").join("portraits")
}

fn image_path(stem: &str) -> PathBuf {
    portraits_dir().join("ff1-pr").join(format!("{stem}.png"))
}

fn read_bytes(path: &PathBuf) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn read_text(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn be_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

fn pixels(stem: &str) -> SharedPixelBuffer<Rgba8Pixel> {
    let bytes = read_bytes(&image_path(stem));
    let image = Image::load_from_data(&bytes, Some("png")).unwrap_or_else(|error| panic!("{stem}.png does not decode: {error:?}"));
    image.to_rgba8().unwrap_or_else(|| panic!("{stem}.png has no pixel buffer"))
}

#[test]
fn every_class_has_a_png_file() {
    for (stem, _) in CLASSES {
        let bytes = read_bytes(&image_path(stem));
        assert!(bytes.len() >= 29, "{stem}.png is too short");
        assert_eq!(bytes[..8], PNG_SIGNATURE, "{stem}.png lacks the PNG signature");
        assert_eq!(&bytes[12..16], b"IHDR", "{stem}.png does not start with IHDR");
        assert_eq!(be_u32(&bytes[16..20]), 32, "{stem}.png width");
        assert_eq!(be_u32(&bytes[20..24]), 32, "{stem}.png height");
        assert_eq!(bytes[24], 8, "{stem}.png bit depth");
        assert_eq!(bytes[25], 6, "{stem}.png colour type");
    }
}

#[test]
fn the_images_decode_to_32_px_with_binary_alpha() {
    for (stem, _) in CLASSES {
        let buffer = pixels(stem);
        assert_eq!((buffer.width(), buffer.height()), (32, 32), "{stem}.png size");
        assert!(
            buffer.as_slice().iter().all(|pixel| pixel.a == 0 || pixel.a == 255),
            "{stem}.png has a partly transparent pixel"
        );
        let slice = buffer.as_slice();
        for index in [0, 31, 31 * 32, 31 * 32 + 31] {
            assert_eq!(slice[index].a, 0, "{stem}.png corner {index} is opaque");
        }
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
