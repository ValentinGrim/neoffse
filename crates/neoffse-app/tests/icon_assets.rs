//! Checks the icon files under `assets/icons/material-symbols/` and their
//! licence. Reads repository files only.

use std::fs;
use std::path::PathBuf;

/// The icons the screens draw, as file stems.
const ICONS: [&str; 12] = [
    "home",
    "arrow_back",
    "add",
    "delete",
    "search",
    "close",
    "expand_more",
    "check",
    "lock",
    "warning",
    "refresh",
    "folder_open",
];

fn icon_path(name: &str) -> PathBuf {
    neoffse_testkit::workspace_root()
        .join("assets")
        .join("icons")
        .join("material-symbols")
        .join(name)
}

fn read_text(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

#[test]
fn every_icon_the_screens_use_exists() {
    for name in ICONS {
        let svg = read_text(&icon_path(&format!("{name}.svg")));
        assert!(svg.starts_with("<svg"), "{name}.svg is not an SVG");
        assert!(svg.contains(r#"viewBox="0 -960 960 960""#), "{name}.svg has another view box");
        // `colorize` sets the colour only when the file fixes none.
        assert!(!svg.contains("fill="), "{name}.svg sets a fill");
    }
}

#[test]
fn the_icon_licence_is_apache_2_0() {
    let licence = read_text(&icon_path("LICENSE"));
    assert!(licence.trim_start().starts_with("Apache License"));
    assert!(licence.contains("Version 2.0"));
}

#[test]
fn the_readme_lists_the_icons() {
    let readme = read_text(&neoffse_testkit::workspace_root().join("licenses").join("README.md"));
    for needle in ["material-symbols", "Apache"] {
        assert!(readme.contains(needle), "licenses/README.md lacks {needle}");
    }
}
