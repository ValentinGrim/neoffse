//! Helpers that several `document_*` test crates share. Each crate declares
//! `mod common;`: Cargo does not build this file as a test target.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use neoffse_format_pr::document::Segment;

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

/// Wraps `doc` in quotes and escapes `\` and `"`, as the game embeds a document.
pub(crate) fn embed(doc: &str) -> String {
    embed_with(doc, false)
}

/// Like [`embed`]; with `unicode_quote` each `"` becomes `"`, a form the
/// game's writer does not produce but a reader must accept.
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
