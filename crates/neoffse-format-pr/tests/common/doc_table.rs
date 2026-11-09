//! A table of `docs/formats/pixel-remaster.md` read as cells, for the tests
//! that pin a Rust table to its document.

/// The cells of every line of the table that follows the heading line
/// `heading` (for example `### Games`), header and separator included,
/// backticks stripped.
pub(crate) fn table_lines(document: &str, heading: &str) -> Vec<Vec<String>> {
    document
        .lines()
        .skip_while(|line| line.trim_end() != heading)
        .skip(1)
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .map(|line| line.trim().trim_matches('|').split('|').map(|cell| cell.trim().replace('`', "")).collect())
        .collect()
}
