//! File helpers for the asset tests: the portrait and icon files under
//! `assets/`. Each asset test crate declares `mod asset_files;`: Cargo does
//! not build this folder as a test target, and the tests do not compile the
//! app rig.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::fs;
use std::path::{Path, PathBuf};

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

/// First eight bytes of every PNG file.
pub(crate) const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Side of every portrait, in pixels.
const PORTRAIT_SIDE: usize = 32;

pub(crate) fn portraits_dir() -> PathBuf {
    neoffse_testkit::workspace_root().join("assets").join("portraits")
}

pub(crate) fn image_path(folder: &str, stem: &str) -> PathBuf {
    portraits_dir().join(folder).join(format!("{stem}.png"))
}

pub(crate) fn read_bytes(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

pub(crate) fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

pub(crate) fn be_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

pub(crate) fn pixels_in(folder: &str, stem: &str) -> SharedPixelBuffer<Rgba8Pixel> {
    let bytes = read_bytes(&image_path(folder, stem));
    let image = Image::load_from_data(&bytes, Some("png")).unwrap_or_else(|error| panic!("{stem}.png does not decode: {error:?}"));
    image.to_rgba8().unwrap_or_else(|| panic!("{stem}.png has no pixel buffer"))
}

/// Checks the format every portrait shares: a 32 by 32 RGBA PNG with 8 bits
/// per channel, binary alpha, and four transparent corners.
pub(crate) fn assert_portrait_png(folder: &str, stem: &str) {
    let bytes = read_bytes(&image_path(folder, stem));
    assert!(bytes.len() >= 29, "{stem}.png is too short");
    assert_eq!(bytes[..8], PNG_SIGNATURE, "{stem}.png lacks the PNG signature");
    assert_eq!(&bytes[12..16], b"IHDR", "{stem}.png does not start with IHDR");
    assert_eq!(be_u32(&bytes[16..20]), 32, "{stem}.png width");
    assert_eq!(be_u32(&bytes[20..24]), 32, "{stem}.png height");
    assert_eq!(bytes[24], 8, "{stem}.png bit depth");
    assert_eq!(bytes[25], 6, "{stem}.png colour type");

    let buffer = pixels_in(folder, stem);
    assert_eq!((buffer.width(), buffer.height()), (32, 32), "{stem}.png size");
    let slice = buffer.as_slice();
    assert!(
        slice.iter().all(|pixel| pixel.a == 0 || pixel.a == 255),
        "{stem}.png has a partly transparent pixel"
    );
    let last = PORTRAIT_SIDE - 1;
    for index in [0, last, last * PORTRAIT_SIDE, last * PORTRAIT_SIDE + last] {
        assert_eq!(slice[index].a, 0, "{stem}.png corner {index} is opaque");
    }
}
