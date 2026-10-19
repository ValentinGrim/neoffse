//! A lossless view of the JSON text that [`container::decode`] returns
//! (`docs/formats/pixel-remaster.md#write-rules`, rule 3).
//!
//! A [`Document`] keeps the raw text of every value. A level is parsed only
//! when a path walks through it, and the JSON that a string holds only when
//! a path asks for it with [`Segment::Doc`]. An edit re-serializes the
//! levels it touched and their parent strings up to the root. A touched
//! level is written compact: the whitespace between its tokens disappears
//! and its keys come back re-escaped by `serde_json`. Untouched members
//! keep their bytes at every level the edit does not cross: number
//! spellings, escapes and whitespace stay as read. A string document on the
//! edit's path is re-serialized as a whole and re-escaped by `serde_json`,
//! which writes only `\"` and `\\` for the characters the game escapes
//! (`docs/formats/pixel-remaster.md#json-text`): an escape in the token of
//! that string, such as `\/` or `\u0022`, comes back canonical. The module
//! knows no domain type.
//!
//! [`container::decode`]: crate::container::decode

mod error;
mod node;
mod scalar;

use std::borrow::Cow;

use serde_json::value::RawValue;

pub use error::DocumentError;
pub use scalar::Scalar;

use node::Node;

/// Longest path, in segments, that a call accepts. The measured saves nest
/// string documents up to 7 levels deep
/// (`docs/formats/pixel-remaster.md#documents-inside-strings`), and 64
/// segments leave room for several keys and indexes per level.
pub const MAX_DEPTH: usize = 64;

/// The four bytes that JSON allows between tokens.
const WHITESPACE: [char; 4] = [' ', '\t', '\n', '\r'];

/// One step of a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment<'a> {
    /// The member `key` of an object.
    Key(&'a str),
    /// The element `index` of an array.
    Index(usize),
    /// Enter the JSON document that the current string holds.
    Doc,
}

/// Renders a path for messages: keys join with `.`, an index shows as
/// `[2]`, and a `Doc` step shows nothing. The empty path, and a path of
/// `Doc` steps, render as `<root>`.
///
/// ```
/// use neoffse_format_pr::document::{Segment, render_path};
///
/// // `userData`, `ownedCharacterList`, each element of `target` and
/// // `parameter` hold JSON strings, so a walk crosses a `Doc` step into each.
/// let path = [
///     Segment::Key("userData"),
///     Segment::Doc,
///     Segment::Key("ownedCharacterList"),
///     Segment::Doc,
///     Segment::Key("target"),
///     Segment::Index(2),
///     Segment::Doc,
///     Segment::Key("parameter"),
/// ];
/// assert_eq!(render_path(&path), "userData.ownedCharacterList.target[2].parameter");
/// assert_eq!(render_path(&[]), "<root>");
/// ```
#[must_use]
pub fn render_path(path: &[Segment<'_>]) -> String {
    let mut out = String::new();
    for segment in path {
        match segment {
            Segment::Key(key) => {
                if !out.is_empty() {
                    out.push('.');
                }
                out.push_str(key);
            }
            Segment::Index(index) => {
                out.push('[');
                out.push_str(&index.to_string());
                out.push(']');
            }
            Segment::Doc => {}
        }
    }
    if out.is_empty() { "<root>".to_owned() } else { out }
}

/// A lossless JSON document: the text around one value, and the value.
///
/// Reads take `&mut self` because they parse levels and keep them. A read
/// never changes [`to_json`](Self::to_json).
///
/// ```
/// use neoffse_format_pr::document::{Document, Scalar, Segment};
///
/// let mut document = Document::parse(r#"{"userData":"{\"playTime\":58.0}"}"#)?;
/// let play_time = [Segment::Key("userData"), Segment::Doc, Segment::Key("playTime")];
///
/// assert_eq!(document.get(&play_time)?, Scalar::Float(58.0));
/// assert_eq!(document.raw(&play_time)?, "58.0"); // the written form survives
/// assert_eq!(document.to_json(), r#"{"userData":"{\"playTime\":58.0}"}"#);
/// # Ok::<(), neoffse_format_pr::document::DocumentError>(())
/// ```
#[derive(Debug, Clone)]
pub struct Document {
    lead: String,
    root: Node,
    tail: String,
}

/// Splits `text` into leading whitespace, the value text and trailing
/// whitespace.
fn split_outer(text: &str) -> (&str, &str, &str) {
    let after_lead = text.trim_start_matches(WHITESPACE);
    let body = after_lead.trim_end_matches(WHITESPACE);
    let lead = text.strip_suffix(after_lead).unwrap_or_default();
    let tail = after_lead.strip_prefix(body).unwrap_or_default();
    (lead, body, tail)
}

/// Parses `body`, the value text of `whole` without its outer whitespace.
/// On failure the error comes from `whole`, so its line and column count
/// from the first byte the caller passed.
fn parse_value(body: &str, whole: &str, path: &[Segment<'_>]) -> Result<Box<RawValue>, DocumentError> {
    RawValue::from_string(body.to_owned()).map_err(|stripped| DocumentError::InvalidJson {
        path: render_path(path),
        source: RawValue::from_string(whole.to_owned()).err().unwrap_or(stripped),
    })
}

fn check_depth(path: &[Segment<'_>]) -> Result<(), DocumentError> {
    if path.len() > MAX_DEPTH {
        Err(DocumentError::TooDeep { limit: MAX_DEPTH })
    } else {
        Ok(())
    }
}

impl Document {
    /// Reads `text` as one JSON value. Leading and trailing whitespace
    /// (space, tab, LF, CR) is kept verbatim. Nothing else is parsed.
    ///
    /// # Errors
    ///
    /// `InvalidJson` with path `<root>` for empty text, whitespace only, a
    /// second value after the first, a truncated value or a leading BOM. Its
    /// line and column count from the first byte of `text`.
    ///
    /// # Examples
    ///
    /// ```
    /// use neoffse_format_pr::document::Document;
    ///
    /// let text = " {\"gil\": 100}\n";
    /// let document = Document::parse(text)?;
    /// assert_eq!(document.to_json(), text); // outer whitespace and inner spacing stay
    /// assert!(Document::parse("").is_err());
    /// assert!(Document::parse("{} {}").is_err());
    /// # Ok::<(), neoffse_format_pr::document::DocumentError>(())
    /// ```
    pub fn parse(text: &str) -> Result<Self, DocumentError> {
        let (lead, body, tail) = split_outer(text);
        Ok(Self {
            lead: lead.to_owned(),
            root: Node::new(parse_value(body, text, &[])?),
            tail: tail.to_owned(),
        })
    }

    /// The document as text: the input byte for byte until an edit touches
    /// it. An edit changes only the levels on its path, and each of those is
    /// written compact.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str(&self.lead);
        self.root.write(&mut out);
        out.push_str(&self.tail);
        out
    }

    /// The exact JSON text of the value at `path`: a string with its quotes
    /// and escapes as written. A value that an edit touched comes back
    /// re-serialized.
    ///
    /// # Errors
    ///
    /// `TooDeep` above [`MAX_DEPTH`] segments, then any step error:
    /// `MissingKey`, `DuplicateKey`, `IndexOutOfRange`, `WrongKind`,
    /// `NotADocument`, `EmptyDocument`, `InvalidJson`.
    pub fn raw(&mut self, path: &[Segment<'_>]) -> Result<Cow<'_, str>, DocumentError> {
        check_depth(path)?;
        Ok(self.root.walk(path, false)?.text())
    }

    /// Classifies the value at `path`. A number with `.`, `e` or `E` reads
    /// as `Float` and loses its written form; use [`raw`](Self::raw) for the
    /// text.
    ///
    /// # Errors
    ///
    /// The errors of [`raw`](Self::raw), plus `WrongKind` for an object or
    /// array, `NumberOutOfRange` and `InvalidJson` for a string with a lone
    /// surrogate escape.
    pub fn get(&mut self, path: &[Segment<'_>]) -> Result<Scalar, DocumentError> {
        check_depth(path)?;
        let node = self.root.walk(path, false)?;
        Scalar::from_text(&node.text(), path)
    }

    /// Writes `value` at `path` (see [`set_raw`](Self::set_raw)). An integer
    /// is plain decimal text and a string is escaped by `serde_json`. A float
    /// is written by `serde_json` (`Number::from_f64`): it keeps `.0` on a
    /// whole value in the range the saves use and switches to an exponent
    /// outside it (`1e16` writes `1e+16`, `1e-7` writes `1e-7`). No number in
    /// the game files uses an exponent
    /// (`docs/formats/pixel-remaster.md#numbers`), so callers keep floats in
    /// that range.
    ///
    /// # Errors
    ///
    /// The errors of [`set_raw`](Self::set_raw), plus `NonFiniteFloat`. The
    /// document stays as it was.
    ///
    /// # Examples
    ///
    /// ```
    /// use neoffse_format_pr::document::{Document, Scalar, Segment};
    ///
    /// let mut document = Document::parse(r#"{"gil": 100, "name": "Firion"}"#)?;
    /// let gil = [Segment::Key("gil")];
    /// document.set(&gil, &Scalar::Int(250))?;
    /// // The level the edit crossed is written compact; the name keeps its text.
    /// assert_eq!(document.to_json(), r#"{"gil":250,"name":"Firion"}"#);
    /// assert_eq!(document.get(&gil)?, Scalar::Int(250));
    ///
    /// assert!(document.set(&[Segment::Key("exp")], &Scalar::Int(1)).is_err());
    /// assert_eq!(document.to_json(), r#"{"gil":250,"name":"Firion"}"#);
    /// # Ok::<(), neoffse_format_pr::document::DocumentError>(())
    /// ```
    pub fn set(&mut self, path: &[Segment<'_>], value: &Scalar) -> Result<(), DocumentError> {
        check_depth(path)?;
        let json = value.json_text(path)?;
        self.set_raw(path, &json)
    }

    /// Replaces the value at `path` with `json`, one valid JSON value of any
    /// kind. Whitespace around `json` is dropped, its inner form is kept. An
    /// empty path replaces the root.
    ///
    /// # Errors
    ///
    /// `TooDeep`, `EndsInDoc` for a path ending in [`Segment::Doc`],
    /// `InvalidJson` for `json`, then any step error. The document stays as
    /// it was.
    ///
    /// # Examples
    ///
    /// ```
    /// use neoffse_format_pr::document::{Document, Segment};
    ///
    /// let mut document = Document::parse(r#"{"list": [1, 2]}"#)?;
    /// let list = [Segment::Key("list")];
    /// // Whitespace around the new value goes; its inner spacing stays.
    /// document.set_raw(&list, " [3,  4] ")?;
    /// assert_eq!(document.to_json(), r#"{"list":[3,  4]}"#);
    ///
    /// assert!(document.set_raw(&list, "[3,").is_err());
    /// assert_eq!(document.to_json(), r#"{"list":[3,  4]}"#);
    /// # Ok::<(), neoffse_format_pr::document::DocumentError>(())
    /// ```
    pub fn set_raw(&mut self, path: &[Segment<'_>], json: &str) -> Result<(), DocumentError> {
        check_depth(path)?;
        if path.last() == Some(&Segment::Doc) {
            return Err(DocumentError::EndsInDoc { path: render_path(path) });
        }
        let (_, body, _) = split_outer(json);
        let value = parse_value(body, json, path)?;
        self.root.walk(path, false)?;
        *self.root.walk(path, true)? = Node::new(value);
        Ok(())
    }
}
