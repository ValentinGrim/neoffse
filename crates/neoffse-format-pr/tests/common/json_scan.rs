//! The scanner behind `json_tree`: reads a text that `serde_json` already
//! accepted as one JSON value and builds its [`Tree`], opening the string
//! documents it finds.

use std::collections::HashSet;

use serde_json::value::RawValue;

use super::json_tree::{Body, Tree};

/// The tree of `text`, one JSON value with no outer whitespace.
pub(super) fn scan(text: &str) -> Tree {
    Scan { text, pos: 0 }.value("")
}

/// The path of member `key` of the node at `path`.
fn join(path: &str, key: &str) -> String {
    if path.is_empty() { key.to_owned() } else { format!("{path}.{key}") }
}

fn shown(path: &str) -> &str {
    if path.is_empty() { "<root>" } else { path }
}

/// A cursor over a text that `serde_json` already accepted as one value, so
/// the scanner only has to find where each token ends.
struct Scan<'a> {
    text: &'a str,
    pos: usize,
}

impl Scan<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }

    fn eat(&mut self, byte: u8) {
        assert_eq!(self.peek(), Some(byte), "invalid JSON");
        self.pos += 1;
    }

    fn ws(&mut self, own: &mut String) {
        while let Some(byte @ (b' ' | b'\t' | b'\n' | b'\r')) = self.peek() {
            own.push(char::from(byte));
            self.pos += 1;
        }
    }

    fn string_token(&mut self) -> &str {
        let start = self.pos;
        self.eat(b'"');
        while let Some(byte) = self.peek() {
            self.pos += if byte == b'\\' { 2 } else { 1 };
            if byte == b'"' {
                break;
            }
        }
        &self.text[start..self.pos]
    }

    fn value(&mut self, path: &str) -> Tree {
        match self.peek() {
            Some(b'{') => self.object(path),
            Some(b'[') => self.array(path),
            Some(b'"') => string_tree(self.string_token(), path),
            _ => {
                let start = self.pos;
                while self
                    .peek()
                    .is_some_and(|byte| !matches!(byte, b',' | b']' | b'}' | b' ' | b'\t' | b'\n' | b'\r'))
                {
                    self.pos += 1;
                }
                Tree::leaf(&self.text[start..self.pos], Body::Scalar)
            }
        }
    }

    fn object(&mut self, path: &str) -> Tree {
        let start = self.pos;
        let mut own = String::new();
        let mut members: Vec<(String, Tree)> = Vec::new();
        self.eat(b'{');
        self.ws(&mut own);
        if self.peek() == Some(b'}') {
            self.pos += 1;
        } else {
            loop {
                let key: String = serde_json::from_str(self.string_token()).expect("invalid JSON");
                self.ws(&mut own);
                self.eat(b':');
                self.ws(&mut own);
                let child = self.value(&join(path, &key));
                members.push((key, child));
                self.ws(&mut own);
                let byte = self.peek();
                self.pos += 1;
                match byte {
                    Some(b',') => self.ws(&mut own),
                    Some(b'}') => break,
                    _ => panic!("invalid JSON"),
                }
            }
        }
        let mut seen = HashSet::new();
        for (key, _) in &members {
            assert!(seen.insert(key.as_str()), "repeated key at {}", shown(&join(path, key)));
        }
        Tree {
            raw: self.text[start..self.pos].to_owned(),
            body: Body::Object(members),
            own_ws: own,
        }
    }

    fn array(&mut self, path: &str) -> Tree {
        let start = self.pos;
        let mut own = String::new();
        let mut items = Vec::new();
        self.eat(b'[');
        self.ws(&mut own);
        if self.peek() == Some(b']') {
            self.pos += 1;
        } else {
            loop {
                items.push(self.value(&format!("{path}[{}]", items.len())));
                self.ws(&mut own);
                let byte = self.peek();
                self.pos += 1;
                match byte {
                    Some(b',') => self.ws(&mut own),
                    Some(b']') => break,
                    _ => panic!("invalid JSON"),
                }
            }
        }
        Tree {
            raw: self.text[start..self.pos].to_owned(),
            body: Body::Array(items),
            own_ws: own,
        }
    }
}

impl Tree {
    fn leaf(raw: &str, body: Body) -> Tree {
        Tree {
            raw: raw.to_owned(),
            body,
            own_ws: String::new(),
        }
    }
}

/// A string node: a document when its content passes the document rule,
/// else a scalar.
fn string_tree(token: &str, path: &str) -> Tree {
    match document_text(token) {
        Some(content) => {
            let inner = Scan { text: &content, pos: 0 }.value(path);
            Tree::leaf(token, Body::Doc(Box::new(inner)))
        }
        None => Tree::leaf(token, Body::Scalar),
    }
}

/// The content of the string `token` when it is one JSON object or array
/// with no padding.
fn document_text(token: &str) -> Option<String> {
    let content: String = serde_json::from_str(token).ok()?;
    if !matches!(content.as_bytes().first(), Some(b'{' | b'[')) {
        return None;
    }
    let raw = RawValue::from_string(content.clone()).ok()?;
    (raw.get().len() == content.len()).then_some(content)
}
