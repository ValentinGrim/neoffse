//! The text encoding of the fake format. Decoding is strict: text that does
//! not re-encode to the same bytes is an error, so one document has one
//! encoding.
#![allow(dead_code, reason = "each test crate uses a subset of the shared helpers")]
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "a test asserts by panicking")]

use std::collections::BTreeSet;
use std::fmt::{self, Write as _};

use neoffse_core::{FieldId, RowId, Value};

use super::fake::{FakeDocument, HEADER};
use super::fake_meta::{Slot, inventory};

/// A decode failure of the fake format.
#[derive(Debug)]
pub(crate) struct FakeError(pub(crate) String);

impl fmt::Display for FakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FakeError {}

/// The storage bound of a row key. The file holds any key in it: the item
/// table is a game rule the codec does not know.
const KEY_BOUNDS: std::ops::RangeInclusive<u32> = 1..=200;

fn fail<T>(message: &str) -> Result<T, FakeError> {
    Err(FakeError(message.to_string()))
}

fn escape(text: &str) -> String {
    text.replace('%', "%25").replace('\n', "%0A").replace('\r', "%0D")
}

fn unescape(text: &str) -> String {
    text.replace("%0D", "\r").replace("%0A", "\n").replace("%25", "%")
}

fn render(value: &Value) -> String {
    match value {
        Value::Int(n) => format!("i:{n}"),
        Value::Bool(b) => format!("b:{b}"),
        Value::Text(t) => format!("t:{}", escape(t)),
        _ => String::new(),
    }
}

fn parse(raw: &str) -> Result<Value, FakeError> {
    if let Some(number) = raw.strip_prefix("i:") {
        return number.parse().map(Value::Int).or_else(|_| fail("bad integer"));
    }
    match raw {
        "b:true" => Ok(Value::Bool(true)),
        "b:false" => Ok(Value::Bool(false)),
        _ => raw
            .strip_prefix("t:")
            .map(|text| Value::Text(unescape(text)))
            .ok_or_else(|| FakeError("bad value".into())),
    }
}

/// The file text: a header line, the hidden text, the scalar fields in id
/// order, then the inventory rows in collection order.
pub(crate) fn encode(doc: &FakeDocument) -> String {
    let mut out = format!("{HEADER}\nhidden={}\n", escape(&doc.hidden));
    for (id, slot) in &doc.scalars {
        writeln!(out, "{id}={}", render(&slot.value)).unwrap();
    }
    for (key, count) in &doc.rows {
        writeln!(out, "{}={}", RowId::new(inventory(), *key).field("count"), render(&Value::Int(*count))).unwrap();
    }
    out
}

fn in_bounds(slot: &Slot, value: &Value) -> bool {
    use neoffse_core::Shape;
    match (slot.meta.shape, value) {
        (Shape::Int { min, max }, Value::Int(n)) => (min..=max).contains(n),
        (Shape::Bool, Value::Bool(_)) | (Shape::Text, Value::Text(_)) => true,
        _ => false,
    }
}

fn row_key(key: &str) -> Option<u32> {
    key.strip_prefix("inventory.")?.strip_suffix(".count")?.parse().ok()
}

pub(crate) fn decode(bytes: &[u8]) -> Result<FakeDocument, FakeError> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return fail("not UTF-8");
    };
    let mut lines = text.split_terminator('\n');
    if lines.next() != Some(HEADER) {
        return fail("bad header");
    }
    let Some(hidden) = lines.next().and_then(|line| line.strip_prefix("hidden=")) else {
        return fail("no hidden line");
    };
    let mut doc = FakeDocument::sample();
    doc.hidden = unescape(hidden);
    doc.rows.clear();
    let ids: Vec<FieldId> = doc.scalars.keys().cloned().collect();
    let mut seen = BTreeSet::new();
    for line in lines {
        let Some((key, raw)) = line.split_once('=') else {
            return fail("line without a value");
        };
        let value = parse(raw)?;
        if let Some(id) = ids.iter().find(|id| id.to_string() == key) {
            let Some(slot) = doc.scalars.get_mut(id) else {
                return fail("unknown field");
            };
            if !seen.insert(id.clone()) || !in_bounds(slot, &value) {
                return fail("repeated field or value out of bounds");
            }
            slot.value = value;
        } else if let (Some(row), Value::Int(count)) = (row_key(key), &value) {
            if !KEY_BOUNDS.contains(&row) || !(1..=99).contains(count) || doc.rows.iter().any(|(k, _)| *k == row) {
                return fail("bad row");
            }
            doc.rows.push((row, *count));
        } else {
            return fail("unknown field");
        }
    }
    if seen.len() != ids.len() {
        return fail("missing field");
    }
    if encode(&doc) != text {
        return fail("not the canonical encoding");
    }
    Ok(doc)
}
