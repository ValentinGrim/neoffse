//! A structural diff of two JSON texts, independent of
//! `neoffse_format_pr::document`. It reports a sorted list of [`Change`]s
//! with a path, so a test states everything an edit changed in a file.
//!
//! A node whose exact text is the same on both sides is unchanged, whatever
//! it holds; every byte the list does not name is therefore identical. A
//! node that differs is explained down to a scalar, an added or removed
//! member or element, a key order change or a whitespace change. A string
//! document is transparent: it adds no path segment
//! (`docs/formats/pixel-remaster.md#documents-inside-strings`). Paths read
//! `userData.ownedCharacterList.target[0].parameter.addtionalLevel`; the root
//! is the empty string and displays as `<root>`. The blind spots of the
//! comparison are listed in `json_tree`.

use std::fmt;

use super::json_tree::{Shape, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    /// Two scalars with different text, or two nodes of different shape.
    Value,
    Added,
    Removed,
    /// The common keys of an object appear in a different order.
    Reordered,
    /// The whitespace of a container, or the escape style of a document's
    /// string token, changed while the content stayed.
    Layout,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Change {
    pub kind: Kind,
    pub path: String,
}

impl fmt::Display for Change {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            Kind::Value => "value",
            Kind::Added => "added",
            Kind::Removed => "removed",
            Kind::Reordered => "reordered",
            Kind::Layout => "layout",
        };
        let path = if self.path.is_empty() { "<root>" } else { &self.path };
        write!(formatter, "{kind} {path}")
    }
}

fn change(kind: Kind, path: &str) -> Change {
    Change { kind, path: path.to_owned() }
}

pub(crate) fn value(path: &str) -> Change {
    change(Kind::Value, path)
}

pub(crate) fn added(path: &str) -> Change {
    change(Kind::Added, path)
}

pub(crate) fn removed(path: &str) -> Change {
    change(Kind::Removed, path)
}

pub(crate) fn reordered(path: &str) -> Change {
    change(Kind::Reordered, path)
}

pub(crate) fn layout(path: &str) -> Change {
    change(Kind::Layout, path)
}

/// The changes from `before` to `after`.
///
/// # Panics
///
/// As [`Tree::parse`].
pub(crate) fn diff(before: &str, after: &str) -> Vec<Change> {
    diff_trees(&Tree::parse(before), &Tree::parse(after))
}

/// Sorted, without duplicate.
pub(crate) fn diff_trees(before: &Tree, after: &Tree) -> Vec<Change> {
    let mut out = Vec::new();
    walk(before, after, "", &mut out);
    out.sort();
    out.dedup();
    out
}

/// The `Layout` changes that an edit at each of `changes` causes in `before`:
/// every strict ancestor of a changed path whose own whitespace is not empty
/// is rewritten compact. The change's own node is not an ancestor. Sorted,
/// without duplicate.
pub(crate) fn layout_chain(before: &Tree, changes: &[Change]) -> Vec<Change> {
    let mut out: Vec<Change> = changes
        .iter()
        .flat_map(|item| ancestors(&item.path))
        .filter(|ancestor| before.at(ancestor).is_some_and(|node| !node.own_ws().is_empty()))
        .map(|ancestor| layout(&ancestor))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Compares two lists in both directions.
///
/// # Panics
///
/// With one line `{label}: unexpected change: {change}` per change only in
/// `actual`, then one line `{label}: missing change: {change}` per change
/// only in `expected`.
pub(crate) fn assert_changes(label: &str, actual: &[Change], expected: &[Change]) {
    let mut actual = actual.to_vec();
    let mut expected = expected.to_vec();
    actual.sort();
    expected.sort();
    let stray = actual.iter().filter(|item| !expected.contains(item));
    let missing = expected.iter().filter(|item| !actual.contains(item));
    let lines: Vec<String> = stray
        .map(|item| format!("{label}: unexpected change: {item}"))
        .chain(missing.map(|item| format!("{label}: missing change: {item}")))
        .collect();
    assert!(lines.is_empty(), "{}", lines.join("\n"));
}

/// `""` first, then the prefix before each `.` and each `[` that follows a
/// name or an index. The path itself is not included.
fn ancestors(path: &str) -> Vec<String> {
    if path.is_empty() {
        return Vec::new();
    }
    let mut out = vec![String::new()];
    out.extend(
        path.char_indices()
            .filter(|&(index, mark)| index > 0 && (mark == '.' || mark == '['))
            .map(|(index, _)| path[..index].to_owned()),
    );
    out
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() { key.to_owned() } else { format!("{path}.{key}") }
}

fn walk(before: &Tree, after: &Tree, path: &str, out: &mut Vec<Change>) {
    if before.raw() == after.raw() {
        return;
    }
    match (before.shape(), after.shape()) {
        (Shape::Object(old), Shape::Object(new)) => {
            objects(old, new, path, out);
            whitespace(before, after, path, out);
        }
        (Shape::Array(old), Shape::Array(new)) => {
            arrays(old, new, path, out);
            whitespace(before, after, path, out);
        }
        (Shape::Doc(old), Shape::Doc(new)) => {
            if old.raw() == new.raw() {
                out.push(layout(path));
            } else {
                walk(old, new, path, out);
            }
        }
        _ => out.push(value(path)),
    }
}

fn whitespace(before: &Tree, after: &Tree, path: &str, out: &mut Vec<Change>) {
    if before.own_ws() != after.own_ws() {
        out.push(layout(path));
    }
}

fn find<'a>(members: &'a [(String, Tree)], key: &str) -> Option<&'a Tree> {
    members.iter().find(|(name, _)| name == key).map(|(_, tree)| tree)
}

fn objects(old: &[(String, Tree)], new: &[(String, Tree)], path: &str, out: &mut Vec<Change>) {
    for (key, _) in old.iter().filter(|(key, _)| find(new, key).is_none()) {
        out.push(removed(&join(path, key)));
    }
    for (key, _) in new.iter().filter(|(key, _)| find(old, key).is_none()) {
        out.push(added(&join(path, key)));
    }
    let common_old = old.iter().map(|(key, _)| key).filter(|key| find(new, key).is_some());
    let common_new = new.iter().map(|(key, _)| key).filter(|key| find(old, key).is_some());
    if !common_old.eq(common_new) {
        out.push(reordered(path));
    }
    for (key, before) in old {
        if let Some(after) = find(new, key) {
            walk(before, after, &join(path, key), out);
        }
    }
}

/// Trims the common prefix and suffix by exact text, pairs the middle from
/// its start and reports the surplus of the longer side.
fn arrays(old: &[Tree], new: &[Tree], path: &str, out: &mut Vec<Change>) {
    let same = |left: &Tree, right: &Tree| left.raw() == right.raw();
    let prefix = old.iter().zip(new).take_while(|(left, right)| same(left, right)).count();
    let room = old.len().min(new.len()) - prefix;
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(room)
        .take_while(|(left, right)| same(left, right))
        .count();
    let (middle_old, middle_new) = (&old[prefix..old.len() - suffix], &new[prefix..new.len() - suffix]);
    for (offset, (before, after)) in middle_old.iter().zip(middle_new).enumerate() {
        walk(before, after, &format!("{path}[{}]", prefix + offset), out);
    }
    let paired = middle_old.len().min(middle_new.len());
    for offset in paired..middle_old.len() {
        out.push(removed(&format!("{path}[{}]", prefix + offset)));
    }
    for offset in paired..middle_new.len() {
        out.push(added(&format!("{path}[{}]", prefix + offset)));
    }
}
