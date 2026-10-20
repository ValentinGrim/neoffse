//! Helpers that the `document_*` test crates and the private-save test
//! crates share. Each crate declares `mod common;`: Cargo does not build
//! this file as a test target.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::path::PathBuf;

pub(crate) mod edit_proof;
mod edit_subject;
mod edit_view;
pub(crate) mod exit_cases;
pub(crate) mod exit_equip;
mod exit_facts;
pub(crate) mod ff1_format;
pub(crate) mod ff1_real;
pub(crate) mod ff1_render;
pub(crate) mod ff1_rules;
pub(crate) mod ff1_save;
pub(crate) mod json_diff;
mod json_scan;
pub(crate) mod json_tree;

use neoffse_format_pr::document::Segment;

/// The six game folders under the saves root, one per game of
/// `docs/formats/pixel-remaster.md#games`.
pub(crate) const GAMES: [&str; 6] = ["ff1", "ff2", "ff3", "ff4", "ff5", "ff6"];

/// Steam writes this file beside the saves; it is not a save
/// (`docs/formats/pixel-remaster.md#other-files-in-the-folder`).
pub(crate) const STEAM_CLOUD_MARKER: &str = "steam_autocloud.vdf";

/// The private save files of `game` without the Steam cloud marker.
/// `None` when the game has no private saves or only the marker; the
/// second case reports the skip through `neoffse_testkit::skip`.
pub(crate) fn save_files(game: &str) -> Option<Vec<PathBuf>> {
    let mut files = neoffse_testkit::private_saves(game)?;
    files.retain(|path| path.file_name().is_none_or(|name| name != STEAM_CLOUD_MARKER));
    if files.is_empty() {
        neoffse_testkit::skip(&format!("{game} holds only the Steam cloud marker"));
        return None;
    }
    Some(files)
}

/// An owned path step, so a test can build paths and borrow them as
/// [`Segment`]s through [`segments`].
#[derive(Debug, Clone)]
pub(crate) enum Step {
    Key(String),
    Index(usize),
    Doc,
}

pub(crate) fn segments(steps: &[Step]) -> Vec<Segment<'_>> {
    steps
        .iter()
        .map(|step| match step {
            Step::Key(key) => Segment::Key(key),
            Step::Index(index) => Segment::Index(*index),
            Step::Doc => Segment::Doc,
        })
        .collect()
}

/// Wraps `doc` in quotes and escapes `\` and `"`, as the game embeds a document
/// (`docs/formats/pixel-remaster.md#documents-inside-strings`).
pub(crate) fn embed(doc: &str) -> String {
    embed_with(doc, false)
}

/// Like [`embed`]; with `unicode_quote` each `"` becomes `\u0022`, a form the
/// game's writer does not produce but a reader must accept: the game writes
/// only `\"` and `\\` escapes (`docs/formats/pixel-remaster.md#json-text`).
pub(crate) fn embed_with(doc: &str, unicode_quote: bool) -> String {
    let quote = if unicode_quote { "\\u0022" } else { "\\\"" };
    format!("\"{}\"", doc.replace('\\', "\\\\").replace('"', quote))
}

/// What remains of `a` and `b` once the common prefix and suffix are gone.
/// Both cuts move back to a char boundary, so multi-byte text never splits.
pub(crate) fn changed_span<'a>(a: &'a str, b: &'a str) -> (&'a str, &'a str) {
    let mut prefix = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
    while !(a.is_char_boundary(prefix) && b.is_char_boundary(prefix)) {
        prefix -= 1;
    }
    let (a, b) = (&a[prefix..], &b[prefix..]);
    let mut suffix = a.bytes().rev().zip(b.bytes().rev()).take_while(|(x, y)| x == y).count();
    while !(a.is_char_boundary(a.len() - suffix) && b.is_char_boundary(b.len() - suffix)) {
        suffix -= 1;
    }
    (&a[..a.len() - suffix], &b[..b.len() - suffix])
}
