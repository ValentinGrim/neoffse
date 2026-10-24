//! Backups built with the io crate in temporary folders.

use std::fs;
use std::path::Path;

use neoffse::Registry;
use neoffse::core::Game;
use neoffse::data::Language;
use neoffse::io::atomic::RealFileOps;
use neoffse::io::backup::{Backup, Moment, Request, create_with};
use neoffse::io::dirs::DataDir;

/// 2020-09-13T12:26:40Z, older than any real clock, so a safety backup made at
/// `Moment::now()` always sorts first.
pub(crate) const T0: i64 = 1_600_000_000;

/// Backs up `files` of `source` at the given instant, `save_type` "slot".
pub(crate) fn make_backup(data: &DataDir, game: &str, source: &Path, files: &[&str], reason: &str, unix_seconds: i64, offset_seconds: i32) -> Backup {
    let request = Request {
        game,
        source_dir: source,
        files,
        save_type: "slot",
        reason,
    };
    let now = Moment::new(unix_seconds, offset_seconds).unwrap();
    create_with(&RealFileOps, data, &request, now).unwrap()
}

/// The readable backups of `game`, newest first, through the facade.
pub(crate) fn listing(data: &DataDir, game: Game) -> Vec<Backup> {
    Registry::new(Language::En).backups(data, game).unwrap().backups
}

/// Replaces the value of `key` in the `backup.json` of `backup`. The file is
/// pretty JSON with one key per line; the value is written as a JSON string.
pub(crate) fn rewrite_manifest(backup: &Backup, key: &str, value: &str) {
    let path = backup.dir.join("backup.json");
    let text = fs::read_to_string(&path).unwrap();
    let prefix = format!("\"{key}\":");
    let mut found = false;
    let lines: Vec<String> = text
        .lines()
        .map(|line| {
            if line.trim_start().starts_with(&prefix) {
                found = true;
                let comma = if line.ends_with(',') { "," } else { "" };
                format!("  {prefix} {}{comma}", serde_text(value))
            } else {
                line.to_owned()
            }
        })
        .collect();
    assert!(found, "the manifest has no key {key}");
    fs::write(&path, lines.join("\n") + "\n").unwrap();
}

/// A JSON string literal: quotes and backslashes escaped.
fn serde_text(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}
