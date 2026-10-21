//! Checks the font files under `assets/fonts/`, their notices and the
//! family names the `Fonts` global holds. Reads repository files only.

use std::fs;
use std::path::PathBuf;

use neoffse_app::{Fonts, MainWindow};
use slint::ComponentHandle;

fn asset(relative: &str) -> PathBuf {
    neoffse_testkit::workspace_root().join("assets").join("fonts").join(relative)
}

fn read(path: &PathBuf) -> Vec<u8> {
    fs::read(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn read_text(path: &PathBuf) -> String {
    String::from_utf8(read(path)).unwrap_or_else(|error| panic!("{} is not UTF-8: {error}", path.display()))
}

fn be16(bytes: &[u8], at: usize) -> Option<usize> {
    let pair = bytes.get(at..at + 2)?;
    Some(usize::from(u16::from_be_bytes([pair[0], pair[1]])))
}

fn be32(bytes: &[u8], at: usize) -> Option<usize> {
    let quad = bytes.get(at..at + 4)?;
    usize::try_from(u32::from_be_bytes([quad[0], quad[1], quad[2], quad[3]])).ok()
}

/// Reads one string of the `name` table of a TrueType file: platform 3
/// (UTF-16BE), else platform 1 (ASCII). `None` when the ID is absent.
fn name_id(font: &[u8], wanted: usize) -> Option<String> {
    let tables = be16(font, 4)?;
    let name_table = (0..tables).find_map(|index| {
        let entry = 12 + index * 16;
        (font.get(entry..entry + 4)? == b"name").then(|| be32(font, entry + 8))?
    })?;
    let count = be16(font, name_table + 2)?;
    let strings = name_table + be16(font, name_table + 4)?;
    let mut found: Option<(usize, String)> = None;
    for index in 0..count {
        let record = name_table + 6 + index * 12;
        if be16(font, record + 6)? != wanted {
            continue;
        }
        let platform = be16(font, record)?;
        let length = be16(font, record + 8)?;
        let start = strings + be16(font, record + 10)?;
        let raw = font.get(start..start + length)?;
        let text = match platform {
            3 => {
                let units: Vec<u16> = raw.as_chunks::<2>().0.iter().map(|pair| u16::from_be_bytes(*pair)).collect();
                String::from_utf16(&units).ok()?
            }
            1 => raw.iter().map(|&byte| char::from(byte)).collect(),
            _ => continue,
        };
        // Platform 3 wins over platform 1.
        if found.as_ref().is_none_or(|(held, _)| platform == 3 && *held != 3) {
            found = Some((platform, text));
        }
    }
    found.map(|(_, text)| text)
}

fn assert_truetype(font: &[u8], label: &str) {
    assert_eq!(
        font.get(..4),
        Some(&[0x00, 0x01, 0x00, 0x00][..]),
        "{label} does not start with the TrueType tag"
    );
}

#[test]
fn the_pixel_font_is_the_one_the_owner_provided() {
    let font = read(&asset("cornelia-sans/cornelia-sans.ttf"));
    assert_eq!(font.len(), 36616);
    assert_truetype(&font, "cornelia-sans.ttf");
}

#[test]
fn the_family_names_match_the_slint_constants() -> Result<(), slint::PlatformError> {
    i_slint_backend_testing::init_no_event_loop();
    let window = MainWindow::new()?;
    let fonts = window.global::<Fonts>();

    let pixel = read(&asset("cornelia-sans/cornelia-sans.ttf"));
    assert_eq!(name_id(&pixel, 1).as_deref(), Some(fonts.get_pixel().as_str()));

    let regular = read(&asset("barlow/Barlow-Regular.ttf"));
    let semibold = read(&asset("barlow/Barlow-SemiBold.ttf"));
    assert_eq!(name_id(&regular, 1).as_deref(), Some(fonts.get_modern().as_str()));
    // Barlow-Regular has no name ID 16; Barlow-SemiBold names the family "Barlow" there,
    // which is why weight 600 on `modern` selects the SemiBold file.
    assert_eq!(name_id(&regular, 16), None);
    assert_eq!(name_id(&semibold, 16).as_deref(), Some(fonts.get_modern_strong().as_str()));
    assert_eq!(fonts.get_modern_strong(), fonts.get_modern());
    assert_eq!(fonts.get_modern_strong_weight(), 600);
    Ok(())
}

#[test]
fn the_chosen_font_files_are_unchanged() {
    for (file, length) in [("Barlow-Regular.ttf", 104_068), ("Barlow-SemiBold.ttf", 108_348)] {
        let font = read(&asset(&format!("barlow/{file}")));
        assert_eq!(font.len(), length, "{file} changed length");
        assert_truetype(&font, file);
    }
    let notice = read_text(&asset("barlow/OFL.txt"));
    assert_eq!(notice.len(), 4377);
    assert!(notice.starts_with("Copyright 2017 The Barlow Project Authors"));
}

#[test]
fn the_licence_notices_are_kept() {
    let fontstruct = read_text(&asset("cornelia-sans/license.txt"));
    for needle in [
        "EliDirkx98",
        "Creative Commons Attribution Share Alike",
        "https://fontstruct.com/fontstructions/show/1914648",
        "http://creativecommons.org/licenses/by-sa/3.0/",
    ] {
        assert!(fontstruct.contains(needle), "license.txt lacks {needle}");
    }
    assert_eq!(fontstruct.lines().count(), 5);
    assert_eq!(fontstruct.lines().last(), Some("[ancestry]"));

    let ofl = read_text(&asset("barlow/OFL.txt"));
    assert!(ofl.contains("SIL OPEN FONT LICENSE Version 1.1"));
}

#[test]
fn the_readme_lists_the_fonts_and_git_treats_ttf_as_binary() {
    let root = neoffse_testkit::workspace_root();
    let readme = read_text(&root.join("licenses").join("README.md"));
    for needle in ["cornelia-sans.ttf", "CC BY-SA 3.0", "barlow", "SIL OFL 1.1"] {
        assert!(readme.contains(needle), "licenses/README.md lacks {needle}");
    }
    let attributes = read_text(&root.join(".gitattributes"));
    assert!(attributes.lines().any(|line| line == "*.ttf binary"));
}
