//! Path helpers of the proof runner: how a failure message shows a path, and
//! the compact-level check of `docs/formats/pixel-remaster.md#write-rules`.

use super::json_diff::Change;
use super::json_tree::Tree;

/// `path` as a failure message shows it: an index equal to the length of its
/// array in `before` reads `[end]`, so an append never prints a length
/// (`docs/architecture.md#test-policy`). Lookups keep the numeric path.
pub(crate) fn shown(before: &Tree, path: &str) -> String {
    let mut out = String::new();
    let mut rest = path;
    while let Some(open) = rest.find('[') {
        let (head, tail) = rest.split_at(open);
        out.push_str(head);
        let Some(close) = tail.find(']') else { break };
        let at_end = tail[1..close].parse::<usize>().is_ok_and(|index| {
            let array = format!("{}{head}", &path[..path.len() - rest.len()]);
            before.array_len(&array) == Some(index)
        });
        out.push_str(if at_end { "[end]" } else { &tail[..=close] });
        rest = &tail[close + 1..];
    }
    out.push_str(rest);
    out
}

/// A change as a failure message shows it, with [`shown`] for its path.
pub(crate) fn describe(before: &Tree, change: &Change) -> String {
    change.describe(&shown(before, &change.path))
}

/// The paths of `chain` that `after` does not hold as compact levels. An edit
/// rewrites every level above it compact, so a level of the `Layout` chain
/// with whitespace of its own, or one that vanished, is a writer fault.
pub(crate) fn spaced_levels(after: &Tree, chain: &[Change]) -> Vec<String> {
    chain
        .iter()
        .filter(|change| after.at(&change.path).is_none_or(|node| !node.own_ws().is_empty()))
        .map(|change| change.path.clone())
        .collect()
}
