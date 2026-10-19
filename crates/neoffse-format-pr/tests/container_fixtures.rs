//! Committed container fixtures under `fixtures/pr/` and the decode-only
//! smoke test on private saves.
//!
//! The JSON is synthetic. Regenerate the files with
//! `NEOFFSE_WRITE_FIXTURES=1 cargo test -p neoffse-format-pr --test container_fixtures write_fixtures`.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::io::Write as _;

use flate2::Compression;
use flate2::write::DeflateEncoder;
use neoffse_format_pr::container::{Container, InflatePath, decode, encode, seal};

struct Fixture {
    name: &'static str,
    json: String,
    container: Container,
    path: InflatePath,
}

/// The slot-index shape: 22 keys, `true` at 1, 21 and 22.
fn slot_index() -> String {
    let keys: Vec<String> = (1..=22).map(|key| key.to_string()).collect();
    let values: Vec<&str> = (1..=22).map(|key| if matches!(key, 1 | 21 | 22) { "true" } else { "false" }).collect();
    format!("{{\"keys\":[{}],\"values\":[{}]}}", keys.join(","), values.join(","))
}

fn large() -> String {
    let items: Vec<String> = (0..600)
        .map(|number| format!("\"{{\\\"contentId\\\":{number},\\\"count\\\":{}}}\"", number % 9 + 1))
        .collect();
    format!("{{\"target\":[{}]}}", items.join(","))
}

fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            name: "game-style.sav",
            json: slot_index(),
            container: Container::game(),
            path: InflatePath::FinalBlock,
        },
        Fixture {
            name: "editor-style.sav",
            json: slot_index(),
            container: Container { bom: true, trailer: vec![] },
            path: InflatePath::FinalBlock,
        },
        Fixture {
            name: "no-bom.sav",
            json: "{\"id\":1,\"name\":\"Hero\",\"gil\":999999}".to_owned(),
            container: Container {
                bom: false,
                trailer: b"\n".to_vec(),
            },
            path: InflatePath::FinalBlock,
        },
        Fixture {
            name: "non-ascii.sav",
            json: "{\"name\":\"Zoé 戦士 ñ\",\"note\":\"café\"}".to_owned(),
            container: Container::game(),
            path: InflatePath::FinalBlock,
        },
        Fixture {
            name: "large.sav",
            json: large(),
            container: Container::game(),
            path: InflatePath::FinalBlock,
        },
        Fixture {
            name: "sync-flushed.sav",
            json: slot_index(),
            container: Container::game(),
            path: InflatePath::Fallback,
        },
    ]
}

fn build(fixture: &Fixture) -> Vec<u8> {
    match fixture.path {
        InflatePath::FinalBlock => encode(&fixture.json, &fixture.container).unwrap(),
        InflatePath::Fallback => {
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(6));
            encoder.write_all(fixture.json.as_bytes()).unwrap();
            seal(&encoder.flush_finish().unwrap(), &fixture.container).unwrap()
        }
    }
}

#[test]
fn committed_fixtures_decode_to_their_expected_json() {
    for fixture in fixtures() {
        let file = neoffse_testkit::read_fixture(&format!("pr/{}", fixture.name)).unwrap();
        let decoded = decode(&file).unwrap_or_else(|error| panic!("{}: {error}", fixture.name));
        assert_eq!(decoded.json, fixture.json, "{}", fixture.name);
        assert_eq!(decoded.container, fixture.container, "{}", fixture.name);
        assert_eq!(decoded.inflate_path, fixture.path, "{}", fixture.name);
    }
}

#[test]
fn fixture_set_covers_the_container_variants() {
    let set = fixtures();
    assert!(set.iter().any(|fixture| fixture.container.bom));
    assert!(set.iter().any(|fixture| !fixture.container.bom));
    assert!(set.iter().any(|fixture| fixture.container.trailer.is_empty()));
    assert!(set.iter().any(|fixture| fixture.container.trailer == b"\n"));
    assert!(set.iter().any(|fixture| fixture.container.trailer == b"\r\n"));
    assert!(set.iter().any(|fixture| !fixture.json.is_ascii()));
    assert!(set.iter().any(|fixture| fixture.json.len() > 15_000));
    assert!(set.iter().any(|fixture| fixture.path == InflatePath::FinalBlock));
    assert!(set.iter().any(|fixture| fixture.path == InflatePath::Fallback));
}

#[test]
fn write_fixtures() {
    if std::env::var_os("NEOFFSE_WRITE_FIXTURES").is_none() {
        return;
    }
    let folder = neoffse_testkit::fixture_path("pr");
    std::fs::create_dir_all(&folder).unwrap();
    for fixture in fixtures() {
        std::fs::write(folder.join(fixture.name), build(&fixture)).unwrap();
    }
}

#[test]
fn private_saves_decode_to_json_objects() {
    for game in common::GAMES {
        let Some(files) = common::save_files(game) else { continue };
        for file in files {
            let name = file.file_name().unwrap().to_string_lossy().into_owned();
            let bytes = std::fs::read(&file).unwrap_or_else(|error| panic!("{game} {name}: {error}"));
            let decoded = decode(&bytes).unwrap_or_else(|error| panic!("{game} {name}: {error}"));
            assert!(decoded.json.starts_with('{'), "{game} {name}: the payload is not an object");
            assert_eq!(decoded.inflate_path, InflatePath::FinalBlock, "{game} {name}");
        }
    }
}
