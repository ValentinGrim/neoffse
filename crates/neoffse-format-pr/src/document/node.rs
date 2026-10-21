use std::borrow::Cow;
use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde_json::value::RawValue;

use super::scalar::string_token;
use super::{DocumentError, Segment, render_path};

/// One JSON value: its exact text, and its parsed level once a path walked
/// through it.
///
/// Invariant: `dirty` implies `body` is `Some`. A clean node writes `raw`
/// unchanged, whatever its form
/// (`docs/formats/pixel-remaster.md#write-rules`, rule 3).
#[derive(Debug, Clone)]
pub(super) struct Node {
    /// The value's text. For a string, the token with its quotes. For the
    /// child of a `Body::Document`, the unescaped content.
    raw: Box<RawValue>,
    body: Option<Body>,
    /// An edit changed something at or below this node.
    dirty: bool,
}

#[derive(Debug, Clone)]
enum Body {
    /// Members in source order, duplicate keys kept.
    Object(Vec<(String, Node)>),
    Array(Vec<Node>),
    /// The document that a string holds
    /// (`docs/formats/pixel-remaster.md#documents-inside-strings`).
    Document(Box<Node>),
}

impl Node {
    pub(super) fn new(raw: Box<RawValue>) -> Self {
        Self { raw, body: None, dirty: false }
    }

    /// The JSON text of the value: the stored text when clean, the
    /// re-serialized text when an edit touched it.
    pub(super) fn text(&self) -> Cow<'_, str> {
        if self.dirty {
            let mut out = String::new();
            self.write(&mut out);
            Cow::Owned(out)
        } else {
            Cow::Borrowed(self.raw.get())
        }
    }

    /// Appends the text of the value. A dirty level is written compact, in
    /// source order, with its duplicate keys. A dirty string document is
    /// written as a new string token around its child's text.
    pub(super) fn write(&self, out: &mut String) {
        match (&self.body, self.dirty) {
            (Some(Body::Object(members)), true) => {
                out.push('{');
                for (index, (key, member)) in members.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(&string_token(key));
                    out.push(':');
                    member.write(out);
                }
                out.push('}');
            }
            (Some(Body::Array(items)), true) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write(out);
                }
                out.push(']');
            }
            (Some(Body::Document(child)), true) => {
                let mut inner = String::new();
                child.write(&mut inner);
                out.push_str(&string_token(&inner));
            }
            _ => out.push_str(self.raw.get()),
        }
    }

    /// The elements of this node, parsed on first use. `walked` is the path
    /// to this node, for errors.
    pub(super) fn array_items(&mut self, walked: &[Segment<'_>]) -> Result<&mut Vec<Node>, DocumentError> {
        let Self { raw, body, .. } = self;
        expect_first(raw.get(), b'[', "array", walked)?;
        if body.is_none() {
            *body = Some(Body::Array(parse_items(raw.get(), walked)?));
        }
        match body {
            Some(Body::Array(items)) => Ok(items),
            _ => Err(wrong_kind(walked, "array")),
        }
    }

    /// Marks an edit at or below this node. Call it on a node whose level an
    /// earlier call parsed, so `dirty` keeps implying `body`: the row
    /// operations call it after the length check, which parsed the array.
    pub(super) fn touch(&mut self) {
        self.dirty = true;
    }

    /// Follows `path` from this node, parsing each level it crosses. With
    /// `mark`, every node left behind turns dirty. A failed walk may leave
    /// nodes dirty, so a caller marks only after a walk with `mark` unset
    /// proved the path.
    pub(super) fn walk(&mut self, path: &[Segment<'_>], mark: bool) -> Result<&mut Self, DocumentError> {
        let mut node = self;
        for (depth, segment) in path.iter().enumerate() {
            node = node.child(*segment, path.get(..depth).unwrap_or_default(), mark)?;
        }
        Ok(node)
    }

    /// One step. `walked` holds the segments before `segment`, for errors.
    fn child(&mut self, segment: Segment<'_>, walked: &[Segment<'_>], mark: bool) -> Result<&mut Self, DocumentError> {
        let Self { raw, body, dirty } = self;
        let text = raw.get();
        match segment {
            Segment::Key(key) => {
                expect_first(text, b'{', "object", walked)?;
                if body.is_none() {
                    *body = Some(Body::Object(parse_members(text, walked)?));
                }
                let Some(Body::Object(members)) = body else {
                    return Err(wrong_kind(walked, "object"));
                };
                let mut found = members.iter_mut().filter(|(name, _)| name == key);
                let first = found.next();
                let repeated = found.next().is_some();
                match first {
                    None => Err(DocumentError::MissingKey {
                        path: render_path(walked),
                        key: key.to_owned(),
                    }),
                    Some(_) if repeated => Err(DocumentError::DuplicateKey {
                        path: render_path(walked),
                        key: key.to_owned(),
                    }),
                    Some((_, member)) => {
                        *dirty |= mark;
                        Ok(member)
                    }
                }
            }
            Segment::Index(index) => {
                expect_first(text, b'[', "array", walked)?;
                if body.is_none() {
                    *body = Some(Body::Array(parse_items(text, walked)?));
                }
                let Some(Body::Array(items)) = body else {
                    return Err(wrong_kind(walked, "array"));
                };
                let len = items.len();
                match items.get_mut(index) {
                    Some(item) => {
                        *dirty |= mark;
                        Ok(item)
                    }
                    None => Err(DocumentError::IndexOutOfRange {
                        path: render_path(walked),
                        index,
                        len,
                    }),
                }
            }
            Segment::Doc => {
                expect_first(text, b'"', "string", walked)?;
                if body.is_none() {
                    *body = Some(Body::Document(Box::new(open_document(text, walked)?)));
                }
                let Some(Body::Document(inner)) = body else {
                    return Err(wrong_kind(walked, "string"));
                };
                *dirty |= mark;
                Ok(&mut **inner)
            }
        }
    }
}

fn wrong_kind(walked: &[Segment<'_>], expected: &'static str) -> DocumentError {
    DocumentError::WrongKind {
        path: render_path(walked),
        expected,
    }
}

fn invalid_json(walked: &[Segment<'_>], source: serde_json::Error) -> DocumentError {
    DocumentError::InvalidJson {
        path: render_path(walked),
        source,
    }
}

/// `RawValue` text never starts with whitespace, so its first byte names the
/// kind.
fn expect_first(text: &str, byte: u8, expected: &'static str, walked: &[Segment<'_>]) -> Result<(), DocumentError> {
    if text.as_bytes().first() == Some(&byte) {
        Ok(())
    } else {
        Err(wrong_kind(walked, expected))
    }
}

/// Object members in source order, duplicates kept. `serde_json::Map` cannot
/// hold a `RawValue`, and an `IndexMap` collapses duplicate keys.
struct Members(Vec<(String, Box<RawValue>)>);

struct MembersVisitor;

impl<'de> Visitor<'de> for MembersVisitor {
    type Value = Members;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Members, A::Error> {
        let mut members = Vec::new();
        while let Some(entry) = map.next_entry::<String, Box<RawValue>>()? {
            members.push(entry);
        }
        Ok(Members(members))
    }
}

impl<'de> Deserialize<'de> for Members {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(MembersVisitor)
    }
}

fn parse_members(text: &str, walked: &[Segment<'_>]) -> Result<Vec<(String, Node)>, DocumentError> {
    let Members(members) = serde_json::from_str(text).map_err(|source| invalid_json(walked, source))?;
    Ok(members.into_iter().map(|(key, raw)| (key, Node::new(raw))).collect())
}

fn parse_items(text: &str, walked: &[Segment<'_>]) -> Result<Vec<Node>, DocumentError> {
    let items: Vec<Box<RawValue>> = serde_json::from_str(text).map_err(|source| invalid_json(walked, source))?;
    Ok(items.into_iter().map(Node::new).collect())
}

/// Unescapes the string token `text` and accepts its content as a document
/// only when the layer can write it back byte for byte.
fn open_document(text: &str, walked: &[Segment<'_>]) -> Result<Node, DocumentError> {
    let content: String = serde_json::from_str(text).map_err(|source| invalid_json(walked, source))?;
    match content.as_bytes().first() {
        None => {
            return Err(DocumentError::EmptyDocument { path: render_path(walked) });
        }
        Some(b'{' | b'[') => {}
        Some(_) => {
            return Err(DocumentError::NotADocument { path: render_path(walked) });
        }
    }
    let len = content.len();
    let raw = RawValue::from_string(content).map_err(|source| invalid_json(walked, source))?;
    if raw.get().len() == len {
        Ok(Node::new(raw))
    } else {
        Err(DocumentError::NotADocument { path: render_path(walked) })
    }
}
