//! Discovery against the real Steam install of the machine the test runs
//! on. The test is ignored by default and only reads. Run it with
//! `cargo test -p neoffse-io --test real_discovery -- --ignored --nocapture`
//! (on Windows: the cross-built test executable with `--ignored --nocapture`).
//!
//! The report and the assertion messages hold game ids, counts and origin
//! kinds only: paths, `SteamID64` values and file names are personal data.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::fs;
use std::path::Path;

use neoffse_format_pr::container::decode;
use neoffse_format_pr::games::GAMES;
use neoffse_io::discovery::{Environment, Origin, SaveFolder, SteamGame, discover};

/// A real save is a few hundred KiB; a larger file is not one.
const MAX_FILE_LEN: u64 = 64 * 1024 * 1024;

/// What discovery found for one game.
struct GameReport {
    id: &'static str,
    library: usize,
    documents: usize,
    other: usize,
    /// Decodable file count of each folder, in discovery order.
    decodable: Vec<usize>,
}

/// Counts the regular files of `folder` that the PR codec accepts. A symlink
/// is not followed, and an unreadable entry counts as not decodable.
fn decodable_files(folder: &Path) -> usize {
    let Ok(entries) = fs::read_dir(folder) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter(|entry| entry.metadata().is_ok_and(|meta| meta.len() <= MAX_FILE_LEN))
        .filter_map(|entry| fs::read(entry.path()).ok())
        .filter(|bytes| decode(bytes).is_ok())
        .count()
}

fn report_for(id: &'static str, folders: &[SaveFolder]) -> GameReport {
    let mine: Vec<&SaveFolder> = folders.iter().filter(|folder| folder.game == id).collect();
    let count = |wanted: fn(&Origin) -> bool| mine.iter().filter(|folder| wanted(&folder.origin)).count();
    let library = count(|origin| matches!(origin, Origin::SteamLibrary { .. }));
    let documents = count(|origin| matches!(origin, Origin::Documents { .. }));
    GameReport {
        id,
        library,
        documents,
        other: mine.len() - library - documents,
        decodable: mine.iter().map(|folder| decodable_files(&folder.path)).collect(),
    }
}

#[allow(clippy::print_stderr, reason = "a manual run prints its counts on stderr")]
fn print_report(reports: &[GameReport], skipped: usize) {
    let mut lines: Vec<String> = reports
        .iter()
        .map(|report| {
            let other = if report.other > 0 {
                format!(", {} other", report.other)
            } else {
                String::new()
            };
            format!(
                "{}: {} folder(s) ({} Steam library, {} Documents{other}), {} decodable file(s)",
                report.id,
                report.decodable.len(),
                report.library,
                report.documents,
                report.decodable.iter().sum::<usize>(),
            )
        })
        .collect();
    lines.push(format!("skipped: {skipped}"));
    eprintln!("{}", lines.join("\n"));
}

#[test]
#[ignore = "reads the real Steam install of this machine"]
fn discovery_finds_a_decodable_save_for_each_of_the_six_games() {
    let games: Vec<SteamGame<'_>> = GAMES
        .iter()
        .map(|game| SteamGame {
            id: game.id,
            app_id: game.app_id,
            folder: game.folder,
        })
        .collect();
    let found = discover(&Environment::platform(), &games);

    let reports: Vec<GameReport> = GAMES.iter().map(|game| report_for(game.id, &found.folders)).collect();
    print_report(&reports, found.skipped.len());

    for report in &reports {
        assert!(!report.decodable.is_empty(), "{}: no save folder found", report.id);
        for (index, count) in report.decodable.iter().enumerate() {
            assert!(*count > 0, "{}: folder {index} holds no decodable file", report.id);
        }
    }
}
