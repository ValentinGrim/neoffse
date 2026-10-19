//! The M1 exit round trips over the real FF I to VI saves.
//!
//! The folder is read-only input and the tests write nothing. A failure names
//! the game and the first 8 characters of the file name, never a value, a
//! length or a decoded byte: the files hold personal data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

mod common;

use std::path::{Path, PathBuf};

use neoffse_format_pr::container::{CodecError, Decoded, InflatePath, decode, encode};
use neoffse_format_pr::document::{Document, DocumentError, Scalar, Segment};

fn label(game: &str, path: &Path) -> String {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    format!("{game} {}", name.chars().take(8).collect::<String>())
}

/// The games that have private saves, with their save files; a game with
/// only the Steam cloud marker counts as absent.
fn present_games() -> Vec<(&'static str, Vec<PathBuf>)> {
    common::GAMES.iter().filter_map(|&game| Some((game, common::save_files(game)?))).collect()
}

fn read(label: &str, path: &Path) -> Vec<u8> {
    let Ok(bytes) = std::fs::read(path) else { panic!("{label}: read failed") };
    bytes
}

fn decoded(label: &str, bytes: &[u8]) -> Decoded {
    let Ok(decoded) = decode(bytes) else { panic!("{label}: decode failed") };
    decoded
}

fn encoded(label: &str, json: &str, decoded: &Decoded) -> Vec<u8> {
    let Ok(file) = encode(json, &decoded.container) else {
        panic!("{label}: encode failed")
    };
    file
}

fn parsed(label: &str, json: &str) -> Document {
    let Ok(document) = Document::parse(json) else {
        panic!("{label}: parse failed")
    };
    document
}

/// Runs `check` on every file of every present game and returns, per game,
/// the file count the listing held and the number of files `check` accepted.
fn for_each_save(mut check: impl FnMut(&str, &str, &[u8]) -> bool) -> Vec<(&'static str, usize, usize)> {
    present_games()
        .into_iter()
        .map(|(game, files)| {
            let accepted = files
                .iter()
                .filter(|path| {
                    let label = label(game, path);
                    let bytes = read(&label, path);
                    check(game, &label, &bytes)
                })
                .count();
            (game, files.len(), accepted)
        })
        .collect()
}

#[test]
fn real_saves_survive_decode_encode_decode() {
    let counts = for_each_save(|_, label, bytes| {
        let d1 = decoded(label, bytes);
        let file = encoded(label, &d1.json, &d1);
        let d2 = decoded(label, &file);
        assert!(d2.json == d1.json, "{label}: json changed");
        assert!(d2.container == d1.container, "{label}: container changed");
        assert!(d2.inflate_path == InflatePath::FinalBlock, "{label}: not the final-block path");
        let bom = neoffse_format_pr::container::text::BOM;
        assert!(file.starts_with(&bom) == bytes.starts_with(&bom), "{label}: BOM changed");
        assert!(file.ends_with(&d1.container.trailer), "{label}: trailer lost");
        assert!(bytes.ends_with(&d1.container.trailer), "{label}: trailer not from the file");
        true
    });
    for (game, listed, accepted) in counts {
        assert!(listed == accepted, "{game}: a file was skipped");
    }
}

#[test]
fn real_saves_survive_the_document_and_the_codec() {
    let counts = for_each_save(|_, label, bytes| {
        let d1 = decoded(label, bytes);
        let document = parsed(label, &d1.json);
        let text = document.to_json();
        assert!(text == d1.json, "{label}: document changed the text");
        let file = encoded(label, &text, &d1);
        let d2 = decoded(label, &file);
        assert!(d2.json == d1.json, "{label}: json changed through the codec");
        true
    });
    for (game, listed, accepted) in counts {
        assert!(listed == accepted, "{game}: a file was skipped");
    }
}

/// The token of the edited key at depth 1: the `userData` string holds the
/// JSON, so its quotes are escaped with a backslash.
fn token(value: i64) -> String {
    format!("\\\"owendGil\\\":{value}")
}

#[test]
#[allow(clippy::match_wild_err_arm, reason = "the error text stays out of the message: privacy")]
fn an_edit_changes_only_its_bytes_through_the_codec() {
    let path = [Segment::Key("userData"), Segment::Doc, Segment::Key("owendGil")];
    let counts = for_each_save(|_, label, bytes| {
        let d1 = decoded(label, bytes);
        let mut document = parsed(label, &d1.json);
        let old = match document.get(&path) {
            Ok(Scalar::Int(value)) => value,
            Ok(Scalar::Uint(value)) => {
                let Ok(value) = i64::try_from(value) else {
                    panic!("{label}: gil out of range")
                };
                value
            }
            Ok(_) => panic!("{label}: gil is not an integer"),
            Err(DocumentError::MissingKey { key, .. }) if key == "userData" => return false,
            Err(_) => panic!("{label}: reading gil failed"),
        };
        // The edit only needs a different value the game would accept.
        let new = if old > 0 { old - 1 } else { 1 };
        assert!(document.set(&path, &Scalar::Int(new)).is_ok(), "{label}: set failed");
        let file = encoded(label, &document.to_json(), &d1);
        let d2 = decoded(label, &file);

        let new_token = token(new);
        assert!(d2.json.matches(&new_token).count() == 1, "{label}: new token not found once");
        let restored = d2.json.replacen(&new_token, &token(old), 1);
        assert!(restored == d1.json, "{label}: the edit changed other bytes");

        let mut reread = parsed(label, &d2.json);
        assert!(
            matches!(reread.get(&path), Ok(Scalar::Int(value)) if value == new)
                || matches!(reread.get(&path), Ok(Scalar::Uint(value)) if i64::try_from(value) == Ok(new)),
            "{label}: the edited value did not read back"
        );
        true
    });
    for (game, _, edited) in counts {
        assert!(edited > 0, "{game}: no file took the edit");
    }
}

#[test]
fn real_saves_decode_with_the_strict_base64() {
    let counts = for_each_save(|_, label, bytes| {
        let result = decode(bytes);
        assert!(!matches!(result, Err(CodecError::Base64 { .. })), "{label}: strict base64 rejected the file");
        assert!(result.is_ok(), "{label}: decode failed");
        true
    });
    for (game, listed, accepted) in counts {
        assert!(listed == accepted, "{game}: a file was skipped");
    }
}
