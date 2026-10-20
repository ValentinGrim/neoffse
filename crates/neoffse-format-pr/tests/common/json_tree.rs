//! A JSON tree that keeps what a writer can damage, for the structural diff
//! of `json_diff`. It reads the text itself and shares no code with
//! `neoffse_format_pr::document`.
//!
//! The tree keeps the exact text of every node, the key order of every
//! object, the whitespace at each container level and the string documents
//! nested in strings (`docs/formats/pixel-remaster.md#documents-inside-strings`).
//! A string is a document when its decoded text starts with `{` or `[` and
//! parses as one JSON value with no padding; any other string is a scalar.
//!
//! Accepted blind spots of the tree and the diff built on it:
//! 1. A key compares by its decoded text, so a writer that re-escaped a key
//!    passes. Keys in the format are ASCII identifiers
//!    (`docs/formats/pixel-remaster.md#key-order-and-spellings`).
//! 2. A container's own whitespace is the ordered run of its whitespace
//!    characters, so a space that moved from one gap to another passes.
//! 3. When the content of a string document changes, the escape style of the
//!    document's own outer token is not checked. The tree parse still checks
//!    that the token decodes to valid JSON.
//! 4. Two equal neighbouring array elements make the reported index of an
//!    insert ambiguous.

use serde_json::Value;

/// What a node holds, borrowed.
pub(crate) enum Shape<'a> {
    Object(&'a [(String, Tree)]),
    Array(&'a [Tree]),
    /// A string token whose content is a JSON document.
    Doc(&'a Tree),
    Scalar,
}

pub(super) enum Body {
    Object(Vec<(String, Tree)>),
    Array(Vec<Tree>),
    Doc(Box<Tree>),
    Scalar,
}

/// One JSON value with its exact text.
pub(crate) struct Tree {
    pub(super) raw: String,
    pub(super) body: Body,
    pub(super) own_ws: String,
}

/// One step of a path: a member name or an element index.
enum Step<'a> {
    Key(&'a str),
    Index(usize),
}

impl Tree {
    /// Reads `json`. Whitespace around the root value counts as the root's
    /// own whitespace.
    ///
    /// # Panics
    ///
    /// With `invalid JSON` when `json` is not one JSON value, and with
    /// `repeated key` and the path of an object that repeats a key.
    pub(crate) fn parse(json: &str) -> Tree {
        let body = json.trim_matches([' ', '\t', '\n', '\r']);
        assert!(serde_json::from_str::<Box<serde_json::value::RawValue>>(body).is_ok(), "invalid JSON");
        let mut tree = super::json_scan::scan(body);
        let lead = json.len() - json.trim_start_matches([' ', '\t', '\n', '\r']).len();
        tree.own_ws.push_str(&json[..lead]);
        tree.own_ws.push_str(&json[lead + body.len()..]);
        json.clone_into(&mut tree.raw);
        tree
    }

    /// The exact text. For a document, the string token with its quotes.
    pub(crate) fn raw(&self) -> &str {
        &self.raw
    }

    /// The whitespace characters at this container's own level, outside
    /// strings and children, in source order. Empty for a compact level, a
    /// scalar and a document.
    pub(crate) fn own_ws(&self) -> &str {
        &self.own_ws
    }

    pub(crate) fn shape(&self) -> Shape<'_> {
        match &self.body {
            Body::Object(members) => Shape::Object(members),
            Body::Array(items) => Shape::Array(items),
            Body::Doc(inner) => Shape::Doc(inner),
            Body::Scalar => Shape::Scalar,
        }
    }

    /// The node at `path`, with every document opened on the way, the last
    /// one included. `""` is the root. `None` for an unknown step.
    pub(crate) fn at(&self, path: &str) -> Option<&Tree> {
        let mut node = self.opened();
        for step in steps(path)? {
            node = match (step, &node.body) {
                (Step::Key(key), Body::Object(members)) => members.iter().find(|(name, _)| name == key).map(|(_, child)| child)?,
                (Step::Index(index), Body::Array(items)) => items.get(index)?,
                _ => return None,
            }
            .opened();
        }
        Some(node)
    }

    pub(crate) fn raw_at(&self, path: &str) -> Option<&str> {
        self.at(path).map(Tree::raw)
    }

    /// The text at `path` as an `i64`. `58.0` and a string give `None`.
    pub(crate) fn int(&self, path: &str) -> Option<i64> {
        self.raw_at(path)?.parse().ok()
    }

    pub(crate) fn array_len(&self, path: &str) -> Option<usize> {
        match &self.at(path)?.body {
            Body::Array(items) => Some(items.len()),
            _ => None,
        }
    }

    /// The index of the first element of the array at `array` whose member
    /// `key` is the integer `wanted`.
    pub(crate) fn find_row(&self, array: &str, key: &str, wanted: i64) -> Option<usize> {
        self.find(array, |element| element.int(key) == Some(wanted))
    }

    /// The index of the first element of the array at `array` that is the
    /// integer `wanted`.
    pub(crate) fn find_int(&self, array: &str, wanted: i64) -> Option<usize> {
        self.find(array, |element| element.int("") == Some(wanted))
    }

    fn find(&self, array: &str, matches: impl Fn(&Tree) -> bool) -> Option<usize> {
        match &self.at(array)?.body {
            Body::Array(items) => items.iter().position(matches),
            _ => None,
        }
    }

    /// The text re-rendered with `": "` and `", "` in every container above
    /// `depth`, compact below. The root is at depth 0 and a document's inner
    /// container sits at the depth of the string that holds it. The original
    /// whitespace is not kept. A document is written as a string token that
    /// `serde_json` escapes.
    pub(crate) fn spaced(&self, depth: usize) -> String {
        let mut out = String::new();
        self.render(0, depth, &mut out);
        out
    }

    fn opened(&self) -> &Tree {
        match &self.body {
            Body::Doc(inner) => inner,
            _ => self,
        }
    }

    fn render(&self, level: usize, limit: usize, out: &mut String) {
        let (colon, comma) = if level < limit { (": ", ", ") } else { (":", ",") };
        match &self.body {
            Body::Object(members) => {
                out.push('{');
                for (index, (key, child)) in members.iter().enumerate() {
                    if index > 0 {
                        out.push_str(comma);
                    }
                    out.push_str(&Value::String(key.clone()).to_string());
                    out.push_str(colon);
                    child.render(level + 1, limit, out);
                }
                out.push('}');
            }
            Body::Array(items) => {
                out.push('[');
                for (index, child) in items.iter().enumerate() {
                    if index > 0 {
                        out.push_str(comma);
                    }
                    child.render(level + 1, limit, out);
                }
                out.push(']');
            }
            Body::Doc(inner) => {
                let mut text = String::new();
                inner.render(level, limit, &mut text);
                out.push_str(&Value::String(text).to_string());
            }
            Body::Scalar => out.push_str(&self.raw),
        }
    }
}

/// Splits `path` on `.` into keys and `[n]` indexes. `None` for a step the
/// syntax does not allow.
fn steps(path: &str) -> Option<Vec<Step<'_>>> {
    let mut out = Vec::new();
    if path.is_empty() {
        return Some(out);
    }
    for part in path.split('.') {
        let (key, mut rest) = part.split_at(part.find('[').unwrap_or(part.len()));
        if key.is_empty() && rest.is_empty() {
            return None;
        }
        if !key.is_empty() {
            out.push(Step::Key(key));
        }
        while let Some(inner) = rest.strip_prefix('[') {
            let (digits, after) = inner.split_once(']')?;
            out.push(Step::Index(digits.parse().ok()?));
            rest = after;
        }
        if !rest.is_empty() {
            return None;
        }
    }
    Some(out)
}
